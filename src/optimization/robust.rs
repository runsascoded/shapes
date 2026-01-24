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
    /// Maximum consecutive rejected steps before giving up (default: 5)
    pub max_rejections: usize,
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
            max_rejections: 5,
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
}

impl Optimizer {
    pub fn new(num_params: usize, config: OptimConfig) -> Self {
        Optimizer {
            config,
            m: vec![0.0; num_params],
            v: vec![0.0; num_params],
            t: 0,
            rejections: 0,
        }
    }

    /// Compute effective learning rate with warmup.
    fn effective_lr(&self) -> f64 {
        if self.t < self.config.warmup_steps {
            // Linear warmup
            self.config.learning_rate * (self.t + 1) as f64 / self.config.warmup_steps as f64
        } else {
            self.config.learning_rate
        }
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
    pub fn should_reject(&mut self, old_error: f64, new_error: f64) -> bool {
        if new_error > old_error * self.config.max_error_increase {
            self.rejections += 1;
            debug!("Rejecting step: error {} -> {} (rejection {})",
                   old_error, new_error, self.rejections);
            true
        } else {
            self.rejections = 0;
            false
        }
    }

    /// Check if we've hit max rejections and should stop.
    pub fn should_stop(&self) -> bool {
        self.rejections >= self.config.max_rejections
    }

    /// Reset rejection counter (call after accepting a step).
    pub fn accept_step(&mut self) {
        self.rejections = 0;
    }

    /// Get current step number.
    pub fn step_count(&self) -> usize {
        self.t
    }
}

/// Train a model using robust optimization.
pub fn train_robust(
    initial_step: &Step,
    config: OptimConfig,
    max_steps: usize,
) -> Result<Vec<Step>, SceneError> {
    let grad_size = initial_step.grad_size();
    let mut optimizer = Optimizer::new(grad_size, config);
    let mut steps = vec![initial_step.clone()];
    let mut current = initial_step.clone();

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
        let new_step = Step::nxt(new_shapes, current.targets.clone())?;
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

        debug!("Step {}: error {:.6} -> {:.6} (lr={:.4})",
               step_idx, current_error, new_error, optimizer.effective_lr());

        steps.push(new_step.clone());
        current = new_step;

        // Check for convergence (error very small or not changing)
        if new_error < 1e-10 {
            info!("Step {}: converged (error < 1e-10)", step_idx);
            break;
        }
    }

    Ok(steps)
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
