use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// Adam optimizer state for gradient descent with momentum.
///
/// Adam (Adaptive Moment Estimation) maintains per-parameter:
/// - First moment estimate (mean of gradients)
/// - Second moment estimate (variance of gradients)
///
/// This helps escape local minima and smooths oscillations that occur
/// with vanilla gradient descent, especially for mixed shape scenes
/// (e.g., polygon + circle) where different parameters have different scales.
#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct AdamState {
    /// First moment estimates (gradient mean), one per parameter
    pub m: Vec<f64>,
    /// Second moment estimates (gradient variance), one per parameter
    pub v: Vec<f64>,
    /// Time step (for bias correction)
    pub t: usize,
    /// Exponential decay rate for first moment (default: 0.9)
    pub beta1: f64,
    /// Exponential decay rate for second moment (default: 0.999)
    pub beta2: f64,
    /// Small constant for numerical stability (default: 1e-8)
    pub epsilon: f64,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct AdamConfig {
    pub beta1: f64,
    pub beta2: f64,
    pub epsilon: f64,
}

impl Default for AdamConfig {
    fn default() -> Self {
        AdamConfig {
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
        }
    }
}

impl AdamState {
    /// Create a new Adam state for `n` parameters.
    pub fn new(n: usize) -> Self {
        Self::with_config(n, AdamConfig::default())
    }

    /// Create a new Adam state with custom hyperparameters.
    pub fn with_config(n: usize, config: AdamConfig) -> Self {
        AdamState {
            m: vec![0.0; n],
            v: vec![0.0; n],
            t: 0,
            beta1: config.beta1,
            beta2: config.beta2,
            epsilon: config.epsilon,
        }
    }

    /// Compute the Adam update step given raw gradients.
    ///
    /// Returns the update vector (to be added to parameters).
    /// The learning rate `alpha` scales the final step.
    pub fn step(&mut self, gradients: &[f64], alpha: f64) -> Vec<f64> {
        self.t += 1;

        // Bias correction factors
        let beta1_correction = 1.0 - self.beta1.powi(self.t as i32);
        let beta2_correction = 1.0 - self.beta2.powi(self.t as i32);

        let mut updates = Vec::with_capacity(gradients.len());

        for (i, &g) in gradients.iter().enumerate() {
            // Update biased first moment estimate
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * g;

            // Update biased second moment estimate
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * g * g;

            // Compute bias-corrected estimates
            let m_hat = self.m[i] / beta1_correction;
            let v_hat = self.v[i] / beta2_correction;

            // Compute update
            let update = alpha * m_hat / (v_hat.sqrt() + self.epsilon);
            updates.push(update);
        }

        updates
    }

    /// Reset the optimizer state (useful when restarting training)
    pub fn reset(&mut self) {
        self.m.fill(0.0);
        self.v.fill(0.0);
        self.t = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adam_basic() {
        let mut adam = AdamState::new(2);

        // Constant gradient should converge to learning_rate per step
        let grad = vec![1.0, 1.0];
        let alpha = 0.1;

        let step1 = adam.step(&grad, alpha);
        let step10 = (1..10).fold(step1.clone(), |_, _| adam.step(&grad, alpha));

        // After warmup, steps should approach alpha (learning rate)
        // m_hat converges to g, v_hat converges to g², so update ≈ alpha * g / |g| = alpha
        assert!(
            (step10[0] - alpha).abs() < 0.01,
            "step10 {} should be close to alpha {}",
            step10[0], alpha
        );
    }

    #[test]
    fn test_adam_oscillating_gradient() {
        let mut adam = AdamState::new(1);
        let alpha = 0.1;

        // Oscillating gradient: momentum should smooth this out
        let steps: Vec<f64> = (0..10)
            .map(|i| {
                let grad = if i % 2 == 0 { vec![1.0] } else { vec![-1.0] };
                adam.step(&grad, alpha)[0]
            })
            .collect();

        // Steps should get smaller as momentum cancels oscillations
        let first_abs = steps[0].abs();
        let last_abs = steps[9].abs();
        assert!(last_abs < first_abs, "oscillations should be dampened: {} vs {}", last_abs, first_abs);
    }
}
