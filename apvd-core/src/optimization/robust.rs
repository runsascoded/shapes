//! Robust optimization with Adam, gradient clipping, and backtracking.
//!
//! This module provides battle-tested optimization that:
//! - Uses Adam for per-parameter adaptive learning rates
//! - Clips gradients to prevent catastrophically large steps
//! - Rejects steps that increase error (simple backtracking)
//! - Supports learning rate warmup for stability

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::error::SceneError;
use crate::step::Step;

/// Configuration for robust optimization.
#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct OptimConfig {
    /// Base learning rate for Adam (default: 0.05)
    pub learning_rate: f64,
    /// Maximum L2 norm for gradient clipping (default: 1.0)
    pub max_grad_norm: f64,
    /// Maximum absolute value for any single gradient component (default: 0.5)
    pub max_grad_value: f64,
    /// Adam beta1 - momentum decay (default: 0.9)
    pub beta1: f64,
    /// Adam beta2 - variance decay (default: 0.999)
    pub beta2: f64,
    /// Adam epsilon for numerical stability (default: 1e-8)
    pub epsilon: f64,
    /// Number of warmup steps (default: 10)
    pub warmup_steps: usize,
    /// Reject steps that increase error by more than this factor (default: 1.5)
    pub max_error_increase: f64,
    /// Maximum consecutive rejected steps before giving up (default: 50)
    pub max_rejections: usize,
    /// LR decay factor per rejection (default: 0.5)
    pub rejection_lr_decay: f64,
}

impl Default for OptimConfig {
    fn default() -> Self {
        OptimConfig {
            learning_rate: 0.05,
            max_grad_norm: 1.0,
            max_grad_value: 0.5,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            warmup_steps: 10,
            max_error_increase: 1.5,
            max_rejections: 50,
            rejection_lr_decay: 0.5,
        }
    }
}

/// Optimizer state for robust training.
#[derive(Clone, Debug)]
pub struct Optimizer {
    config: OptimConfig,
    /// First moment estimates (gradient mean)
    m: Vec<f64>,
    /// Second moment estimates (gradient variance)
    v: Vec<f64>,
    /// Time step
    t: usize,
    /// Consecutive rejections
    rejections: usize,
    /// Current LR scale factor (reduced on rejection, restored on accept)
    lr_scale: f64,
}

impl Optimizer {
    pub fn new(num_params: usize, config: OptimConfig) -> Self {
        Optimizer {
            config,
            m: vec![0.0; num_params],
            v: vec![0.0; num_params],
            t: 0,
            rejections: 0,
            lr_scale: 1.0,
        }
    }

    /// Compute effective learning rate with warmup and rejection decay.
    pub fn effective_lr(&self) -> f64 {
        let base = if self.t < self.config.warmup_steps {
            // Linear warmup
            self.config.learning_rate * (self.t + 1) as f64 / self.config.warmup_steps as f64
        } else {
            self.config.learning_rate
        };
        base * self.lr_scale
    }

    /// Clip gradients by value and L2 norm.
    fn clip_gradients(&self, grads: &[f64]) -> Vec<f64> {
        let mut clipped: Vec<f64> = grads.iter()
            .map(|&g| g.clamp(-self.config.max_grad_value, self.config.max_grad_value))
            .collect();

        // Also clip by L2 norm
        let norm: f64 = clipped.iter().map(|g| g * g).sum::<f64>().sqrt();
        if norm > self.config.max_grad_norm {
            let scale = self.config.max_grad_norm / norm;
            for g in &mut clipped {
                *g *= scale;
            }
        }

        clipped
    }

    /// Compute Adam update for given gradients.
    /// Returns the parameter updates (to be added to current params).
    pub fn compute_update(&mut self, raw_grads: &[f64]) -> Vec<f64> {
        self.t += 1;

        // Clip gradients first
        let grads = self.clip_gradients(raw_grads);

        let lr = self.effective_lr();
        let beta1_correction = 1.0 - self.config.beta1.powi(self.t as i32);
        let beta2_correction = 1.0 - self.config.beta2.powi(self.t as i32);

        let mut updates = Vec::with_capacity(grads.len());

        for (i, &g) in grads.iter().enumerate() {
            // Update biased first moment estimate
            self.m[i] = self.config.beta1 * self.m[i] + (1.0 - self.config.beta1) * g;

            // Update biased second moment estimate
            self.v[i] = self.config.beta2 * self.v[i] + (1.0 - self.config.beta2) * g * g;

            // Compute bias-corrected estimates
            let m_hat = self.m[i] / beta1_correction;
            let v_hat = self.v[i] / beta2_correction;

            // Compute update
            let update = lr * m_hat / (v_hat.sqrt() + self.config.epsilon);
            updates.push(update);
        }

        updates
    }

    /// Check if a step should be rejected (error increased too much).
    /// On rejection, decays the LR and resets Adam momentum to escape stale gradients.
    pub fn should_reject(&mut self, old_error: f64, new_error: f64) -> bool {
        if new_error > old_error * self.config.max_error_increase {
            self.rejections += 1;
            self.lr_scale *= self.config.rejection_lr_decay;
            // Reset Adam momentum to prevent stale momentum from dominating
            for m in &mut self.m { *m = 0.0; }
            debug!("Rejecting step: error {} -> {} (rejection {}, lr_scale={:.4})",
                   old_error, new_error, self.rejections, self.lr_scale);
            true
        } else {
            false
        }
    }

    /// Check if we've hit max rejections and should stop.
    pub fn should_stop(&self) -> bool {
        self.rejections >= self.config.max_rejections
    }

    /// Reset rejection counter and restore LR (call after accepting a step).
    pub fn accept_step(&mut self) {
        if self.rejections > 0 {
            // Gradually restore LR (don't snap back to full)
            self.lr_scale = (self.lr_scale * 2.0).min(1.0);
        }
        self.rejections = 0;
    }

    /// Get current step number.
    pub fn step_count(&self) -> usize {
        self.t
    }
}

/// Result of robust training with optional step filtering.
pub struct RobustResult {
    /// Retained steps (filtered by predicate + best + final).
    pub steps: Vec<Step>,
    /// Original (absolute) step index for each retained step.
    pub step_indices: Vec<usize>,
    /// Total iterations actually completed (accepted steps only).
    pub total_steps: usize,
    /// Absolute index of the best (min error) step.
    pub min_idx: usize,
    /// Best error achieved.
    pub min_error: f64,
}

/// Train using robust optimization, retaining all steps.
pub fn train_robust(
    initial_step: &Step,
    config: OptimConfig,
    max_steps: usize,
) -> Result<RobustResult, SceneError> {
    train_robust_filtered(initial_step, config, max_steps, 0, None)
}

/// Train using robust optimization with optional step filtering.
///
/// `step_offset`: added to internal indices so `retain` sees absolute step numbers.
/// `retain`: if `Some`, only steps where `retain(abs_idx)` returns true are kept
/// in the result (plus the best and final steps, which are always retained).
pub fn train_robust_filtered(
    initial_step: &Step,
    config: OptimConfig,
    max_steps: usize,
    step_offset: usize,
    retain: Option<&dyn Fn(usize) -> bool>,
) -> Result<RobustResult, SceneError> {
    let grad_size = initial_step.grad_size();
    let mut optimizer = Optimizer::new(grad_size, config);

    // Always retain the initial step
    let mut steps = vec![initial_step.clone()];
    let mut step_indices = vec![step_offset];

    let mut current = initial_step.clone();
    let mut min_idx = step_offset;
    let mut min_error = initial_step.error.v();
    // Track best step separately so we can ensure it's in the output
    let mut best_step: Option<Step> = None;

    // Count accepted steps (excluding initial)
    let mut accepted_count: usize = 0;

    for step_idx in 0..max_steps {
        let error = current.error.clone();
        let current_error = error.v();

        // Get gradient (negative because we want to minimize)
        let grad_vec = (-error).d();

        // Skip if gradient is zero or NaN
        let grad_magnitude: f64 = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();
        if grad_magnitude == 0.0 || grad_magnitude.is_nan() {
            debug!("Step {}: zero/NaN gradient, stopping", step_idx);
            break;
        }

        // Compute Adam update
        let updates = optimizer.compute_update(&grad_vec);

        // Apply updates to get new shapes
        let new_shapes: Vec<_> = current.shapes.iter()
            .map(|s| s.step(&updates))
            .collect();

        // Compute new step
        let new_step = Step::nxt(new_shapes, current.targets.clone(), current.penalty_config.clone())?;
        let new_error = new_step.error.v();

        // Check for NaN
        if new_error.is_nan() {
            warn!("Step {}: NaN error, stopping", step_idx);
            break;
        }

        // Check if we should reject this step
        if optimizer.should_reject(current_error, new_error) {
            if optimizer.should_stop() {
                info!("Step {}: max rejections reached, stopping", step_idx);
                break;
            }
            // Don't update current, try again with decayed update
            continue;
        }

        optimizer.accept_step();
        accepted_count += 1;
        let abs_idx = step_offset + accepted_count;

        debug!("Step {}: error {:.6} -> {:.6} (lr={:.4})",
               step_idx, current_error, new_error, optimizer.effective_lr());

        // Track best
        if new_error < min_error {
            min_error = new_error;
            min_idx = abs_idx;
            best_step = Some(new_step.clone());
        }

        // Decide whether to retain this step
        let should_retain = match retain {
            Some(f) => f(abs_idx),
            None => true,
        };
        if should_retain {
            steps.push(new_step.clone());
            step_indices.push(abs_idx);
        }

        current = new_step;

        // Check for convergence (error + penalties very small)
        if new_error + current.penalties.total() < 1e-10 {
            info!("Step {}: converged (error + penalties < 1e-10)", step_idx);
            break;
        }
    }

    let total_steps = step_offset + accepted_count + 1; // +1 for initial step

    // Ensure best step is in the output
    if let Some(best) = best_step {
        if !step_indices.contains(&min_idx) {
            steps.push(best);
            step_indices.push(min_idx);
        }
    }

    // Ensure final step is in the output
    let final_abs_idx = step_offset + accepted_count;
    if accepted_count > 0 && !step_indices.contains(&final_abs_idx) {
        steps.push(current);
        step_indices.push(final_abs_idx);
    }

    // Sort by step index to maintain order
    let mut indexed: Vec<_> = step_indices.into_iter().zip(steps.into_iter()).collect();
    indexed.sort_by_key(|(idx, _)| *idx);
    let (step_indices, steps): (Vec<_>, Vec<_>) = indexed.into_iter().unzip();

    Ok(RobustResult {
        steps,
        step_indices,
        total_steps,
        min_idx,
        min_error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_clipping() {
        let config = OptimConfig {
            max_grad_value: 0.5,
            max_grad_norm: 1.0,
            ..Default::default()
        };
        let optimizer = Optimizer::new(3, config);

        // Test value clipping
        let grads = vec![2.0, -3.0, 0.1];
        let clipped = optimizer.clip_gradients(&grads);
        assert!(clipped[0] <= 0.5);
        assert!(clipped[1] >= -0.5);

        // Test norm clipping
        let grads = vec![0.4, 0.4, 0.4, 0.4, 0.4]; // norm = ~0.89
        let optimizer = Optimizer::new(5, OptimConfig {
            max_grad_value: 1.0,
            max_grad_norm: 0.5,
            ..Default::default()
        });
        let clipped = optimizer.clip_gradients(&grads);
        let clipped_norm: f64 = clipped.iter().map(|g| g * g).sum::<f64>().sqrt();
        assert!(clipped_norm <= 0.5 + 1e-10);
    }

    #[test]
    fn test_warmup() {
        let config = OptimConfig {
            learning_rate: 0.1,
            warmup_steps: 10,
            ..Default::default()
        };
        let mut optimizer = Optimizer::new(1, config);

        // During warmup, LR should increase linearly
        let grads = vec![1.0];
        optimizer.compute_update(&grads);
        assert!((optimizer.effective_lr() - 0.02).abs() < 1e-10); // step 1: 0.1 * 2/10

        for _ in 0..9 {
            optimizer.compute_update(&grads);
        }
        assert!((optimizer.effective_lr() - 0.1).abs() < 1e-10); // step 10: full LR
    }
}
