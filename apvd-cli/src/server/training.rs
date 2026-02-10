//! Training loop implementations (single and parallel).

use std::sync::Arc;

use rayon::prelude::*;
use tokio::sync::mpsc;

use apvd_core::{InputSpec, Model, TargetsMap};

use super::progress::{send_progress_notification, send_progress_update, send_progress_update_with_type};
use super::session::TrainingSession;

pub(super) fn run_single_training(
    shapes: Vec<InputSpec>,
    targets: TargetsMap<f64>,
    max_steps: usize,
    learning_rate: f64,
    robust: bool,
    session: Arc<TrainingSession>,
    tx: mpsc::Sender<String>,
    handle_id: String,
) {
    let start_time = std::time::Instant::now();

    // Create model
    let model_result = Model::new(shapes, targets.clone());
    let mut model = match model_result {
        Ok(m) => m,
        Err(e) => {
            send_progress_notification(&tx, &handle_id, "error", 0, max_steps, 0.0, 0.0, 0, vec![], start_time.elapsed().as_millis() as u64, false, Some(e.to_string()));
            return;
        }
    };

    // Record and send initial step
    let initial_step = model.steps.last().unwrap();
    session.record_step(0, initial_step.clone(), initial_step.error.v());
    send_progress_update(&tx, &handle_id, &model, 0, max_steps, start_time);

    // Training loop
    for step_idx in 0..max_steps {
        // Check for stop request
        if session.should_stop() {
            return;
        }

        let current_step = model.steps.last().unwrap();

        // Check for convergence
        if current_step.converged {
            break;
        }

        // Take a step
        let next_step = if robust {
            current_step.step_clipped(learning_rate, 0.5, 1.0)
        } else {
            current_step.step(learning_rate)
        };

        match next_step {
            Ok(step) => {
                let err = step.error.v();
                if err.is_nan() {
                    send_progress_notification(&tx, &handle_id, "error", step_idx, max_steps, err, model.min_error, model.min_idx, vec![], start_time.elapsed().as_millis() as u64, false, Some("NaN error encountered".to_string()));
                    break;
                }

                // Record step in trace storage
                let new_step_idx = model.steps.len();
                session.record_step(new_step_idx, step.clone(), err);

                // Update min tracking
                if err < model.min_error {
                    model.min_idx = new_step_idx;
                    model.min_error = err;
                }

                model.steps.push(step);

                // Send update every 10 steps or on significant changes
                if step_idx % 10 == 0 || step_idx < 20 {
                    send_progress_update(&tx, &handle_id, &model, step_idx + 1, max_steps, start_time);
                }
            }
            Err(e) => {
                send_progress_notification(&tx, &handle_id, "error", step_idx, max_steps, 0.0, model.min_error, model.min_idx, vec![], start_time.elapsed().as_millis() as u64, false, Some(format!("Step failed: {}", e)));
                break;
            }
        }
    }

    if session.should_stop() {
        return;
    }

    // Send completion
    send_progress_update_with_type(&tx, &handle_id, &model, model.steps.len() - 1, max_steps, start_time, "complete");
}

pub(super) fn run_parallel_training(
    shapes: Vec<InputSpec>,
    targets: TargetsMap<f64>,
    max_steps: usize,
    learning_rate: f64,
    robust: bool,
    num_parallel: usize,
    session: Arc<TrainingSession>,
    tx: mpsc::Sender<String>,
    handle_id: String,
) {
    let start_time = std::time::Instant::now();
    let num_shapes = shapes.len();
    let permutations = generate_permutations(num_shapes, num_parallel);

    // Shared state for tracking best result
    let best_result: Arc<std::sync::Mutex<Option<(Vec<usize>, Model)>>> =
        Arc::new(std::sync::Mutex::new(None));

    // Train all variants in parallel
    permutations.into_par_iter().enumerate().for_each(|(variant_id, permutation)| {
        if session.should_stop() {
            return;
        }

        // Reorder inputs according to permutation
        let reordered_inputs: Vec<InputSpec> = permutation
            .iter()
            .map(|&idx| shapes[idx].clone())
            .collect();

        // Create model
        let model_result = Model::new(reordered_inputs, targets.clone());
        let mut model = match model_result {
            Ok(m) => m,
            Err(_) => return,
        };

        // Training loop
        for step_idx in 0..max_steps {
            if session.should_stop() {
                return;
            }

            let current_step = model.steps.last().unwrap();

            if current_step.converged {
                break;
            }

            let next_step = if robust {
                current_step.step_clipped(learning_rate, 0.5, 1.0)
            } else {
                current_step.step(learning_rate)
            };

            match next_step {
                Ok(step) => {
                    let err = step.error.v();
                    if err.is_nan() {
                        break;
                    }

                    let new_step_idx = model.steps.len();

                    if err < model.min_error {
                        model.min_idx = new_step_idx;
                        model.min_error = err;
                    }

                    // Check if this is now the best variant
                    if session.update_best(variant_id, err) {
                        // Record step from best variant
                        session.record_step(new_step_idx, step.clone(), err);
                        // Send update (only from best variant)
                        if step_idx % 10 == 0 || step_idx < 20 {
                            send_progress_update(&tx, &handle_id, &model, step_idx + 1, max_steps, start_time);
                        }
                    }

                    model.steps.push(step);
                }
                Err(_) => break,
            }
        }

        // Update best result if this variant is best
        let final_error = model.steps.last().map(|s| s.error.v()).unwrap_or(f64::INFINITY);
        let mut best = best_result.lock().unwrap();
        if best.is_none() || final_error < best.as_ref().unwrap().1.min_error {
            *best = Some((permutation, model));
        }
    });

    if session.should_stop() {
        return;
    }

    // Send final result
    let best = best_result.lock().unwrap();
    if let Some((_permutation, model)) = best.as_ref() {
        send_progress_update_with_type(&tx, &handle_id, model, model.steps.len() - 1, max_steps, start_time, "complete");
    } else {
        send_progress_notification(&tx, &handle_id, "error", 0, max_steps, 0.0, 0.0, 0, vec![], start_time.elapsed().as_millis() as u64, false, Some("All training variants failed".to_string()));
    }
}

/// Generate shape permutations for parallel training.
pub(super) fn generate_permutations(n: usize, max_count: usize) -> Vec<Vec<usize>> {
    if max_count == 1 {
        return vec![(0..n).collect()];
    }

    let mut permutations = Vec::new();
    let mut arr: Vec<usize> = (0..n).collect();

    fn heap_permute(k: usize, arr: &mut Vec<usize>, result: &mut Vec<Vec<usize>>) {
        if k == 1 {
            result.push(arr.clone());
            return;
        }
        heap_permute(k - 1, arr, result);
        for i in 0..k - 1 {
            if k % 2 == 0 {
                arr.swap(i, k - 1);
            } else {
                arr.swap(0, k - 1);
            }
            heap_permute(k - 1, arr, result);
        }
    }

    heap_permute(n, &mut arr, &mut permutations);

    if permutations.len() > max_count {
        let step = permutations.len() / max_count;
        permutations = permutations
            .into_iter()
            .step_by(step)
            .take(max_count)
            .collect();
    }

    permutations
}
