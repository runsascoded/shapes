//! CLI and server for area-proportional Venn diagrams.
//!
//! Provides:
//! - Batch training from command line
//! - WebSocket server for frontend connections
//! - Parallel scene training across different initial assignments

use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use clap::{Parser, Subcommand};
use rayon::prelude::*;
use serde::Serialize;

use apvd_core::{Model, InputSpec, TargetsMap};

#[derive(Parser)]
#[command(name = "apvd")]
#[command(about = "Area-proportional Venn diagram generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Train a model from the command line
    Train {
        /// Input shapes (JSON file or inline JSON)
        #[arg(short, long)]
        shapes: String,

        /// Target areas (JSON file or inline JSON)
        #[arg(short, long)]
        targets: String,

        /// Maximum training steps
        #[arg(short, long, default_value = "1000")]
        max_steps: usize,

        /// Learning rate
        #[arg(short, long, default_value = "0.05")]
        learning_rate: f64,

        /// Number of parallel scene variants to train (permutations of shape assignments)
        #[arg(short, long, default_value = "1")]
        parallel: usize,

        /// Output file for results (JSON). If not specified, prints to stdout.
        #[arg(short, long)]
        output: Option<String>,

        /// Use robust optimizer (Adam + clipping + backtracking)
        #[arg(short = 'R', long)]
        robust: bool,

        /// Quiet mode - only output final JSON
        #[arg(short, long)]
        quiet: bool,
    },

    /// Start WebSocket server for frontend connections
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Number of parallel scene variants to train
        #[arg(long, default_value = "1")]
        parallel: usize,
    },
}

/// Result of training a single scene variant
#[derive(Debug, Clone, Serialize)]
struct TrainingResult {
    /// Which permutation of shapes was used (index into all permutations)
    variant_id: usize,
    /// The permutation used (maps target index to shape index)
    permutation: Vec<usize>,
    /// Final error after training
    final_error: f64,
    /// Minimum error achieved during training
    min_error: f64,
    /// Step index where minimum error was achieved
    min_step: usize,
    /// Total steps taken
    total_steps: usize,
    /// Training time in milliseconds
    training_time_ms: u64,
    /// Final shapes (serialized)
    final_shapes: Vec<serde_json::Value>,
}

/// Combined output for all training variants
#[derive(Debug, Serialize)]
struct TrainingOutput {
    /// Best result (lowest final error)
    best: TrainingResult,
    /// All results, sorted by final error
    all_results: Vec<TrainingResult>,
    /// Total wall-clock time in milliseconds
    total_time_ms: u64,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Train {
            shapes,
            targets,
            max_steps,
            learning_rate,
            parallel,
            output,
            robust,
            quiet,
        } => {
            if let Err(e) = run_train(shapes, targets, max_steps, learning_rate, parallel, output, robust, quiet) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Serve { port, parallel } => {
            eprintln!("Server mode not yet implemented");
            eprintln!("  port: {}", port);
            eprintln!("  parallel: {}", parallel);
            std::process::exit(1);
        }
    }
}

fn run_train(
    shapes_arg: String,
    targets_arg: String,
    max_steps: usize,
    learning_rate: f64,
    parallel: usize,
    output: Option<String>,
    robust: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    // Parse shapes (from file or inline JSON)
    let shapes_json = read_json_arg(&shapes_arg)?;
    let inputs: Vec<InputSpec> = serde_json::from_str(&shapes_json)
        .map_err(|e| format!("Failed to parse shapes JSON: {}", e))?;

    // Parse targets (from file or inline JSON)
    let targets_json = read_json_arg(&targets_arg)?;
    let targets: TargetsMap<f64> = serde_json::from_str(&targets_json)
        .map_err(|e| format!("Failed to parse targets JSON: {}", e))?;

    let num_shapes = inputs.len();
    if !quiet {
        eprintln!("Training with {} shapes, {} target regions", num_shapes, targets.len());
        eprintln!("  max_steps: {}, learning_rate: {}", max_steps, learning_rate);
        eprintln!("  optimizer: {}", if robust { "robust (Adam + clipping)" } else { "standard GD" });
    }

    // Generate permutations for parallel training
    let permutations = generate_permutations(num_shapes, parallel);
    let num_variants = permutations.len();

    if !quiet && num_variants > 1 {
        eprintln!("Training {} variants in parallel", num_variants);
    }

    // Progress tracking
    let completed = Arc::new(AtomicUsize::new(0));

    // Train all variants in parallel
    let results: Vec<TrainingResult> = permutations
        .into_par_iter()
        .enumerate()
        .map(|(variant_id, permutation)| {
            let variant_start = Instant::now();

            // Reorder inputs according to permutation
            let reordered_inputs: Vec<InputSpec> = permutation
                .iter()
                .map(|&idx| inputs[idx].clone())
                .collect();

            // Create and train model
            let mut model = Model::new(reordered_inputs, targets.clone())
                .expect("Failed to create model");

            if robust {
                model.train_robust(max_steps).expect("Training failed");
            } else {
                model.train(learning_rate, max_steps).expect("Training failed");
            }

            let training_time_ms = variant_start.elapsed().as_millis() as u64;

            // Extract results
            let final_step = model.steps.last().unwrap();
            let final_shapes: Vec<serde_json::Value> = final_step
                .shapes
                .iter()
                .map(|s| serde_json::to_value(s).unwrap())
                .collect();

            let result = TrainingResult {
                variant_id,
                permutation: permutation.clone(),
                final_error: final_step.error.v(),
                min_error: model.min_error,
                min_step: model.min_idx,
                total_steps: model.steps.len(),
                training_time_ms,
                final_shapes,
            };

            // Update progress
            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
            if !quiet && num_variants > 1 {
                eprint!("\rCompleted {}/{} variants", done, num_variants);
                io::stderr().flush().ok();
            }

            result
        })
        .collect();

    if !quiet && num_variants > 1 {
        eprintln!(); // Newline after progress
    }

    // Sort by final error and find best
    let mut sorted_results = results;
    sorted_results.sort_by(|a, b| a.final_error.partial_cmp(&b.final_error).unwrap());

    let best = sorted_results[0].clone();
    let total_time_ms = start_time.elapsed().as_millis() as u64;

    if !quiet {
        eprintln!("\nBest result: variant {} with error {:.6e}", best.variant_id, best.final_error);
        eprintln!("  permutation: {:?}", best.permutation);
        eprintln!("  min_error: {:.6e} at step {}", best.min_error, best.min_step);
        eprintln!("  total time: {}ms", total_time_ms);
    }

    let output_data = TrainingOutput {
        best,
        all_results: sorted_results,
        total_time_ms,
    };

    // Output results
    let json_output = serde_json::to_string_pretty(&output_data)?;
    if let Some(output_path) = output {
        std::fs::write(&output_path, &json_output)?;
        if !quiet {
            eprintln!("Results written to {}", output_path);
        }
    } else {
        println!("{}", json_output);
    }

    Ok(())
}

/// Read JSON from a file path or treat the argument as inline JSON
fn read_json_arg(arg: &str) -> Result<String, Box<dyn std::error::Error>> {
    // If it starts with '{' or '[', treat as inline JSON
    let trimmed = arg.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        Ok(arg.to_string())
    } else {
        // Try to read as file
        std::fs::read_to_string(arg)
            .map_err(|e| format!("Failed to read file '{}': {}", arg, e).into())
    }
}

/// Generate shape permutations for parallel training.
///
/// For n shapes, there are n! permutations, but many may be equivalent due to
/// symmetry in the initial layout. This function generates up to `max_count`
/// distinct permutations.
///
/// For 4 shapes with our typical 2+2 symmetric layout, there are effectively
/// only 6 distinguishable assignments (4C2 = 6).
fn generate_permutations(n: usize, max_count: usize) -> Vec<Vec<usize>> {
    if max_count == 1 {
        // Just use identity permutation
        return vec![(0..n).collect()];
    }

    // Generate all permutations using Heap's algorithm
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

    // Limit to max_count
    if permutations.len() > max_count {
        // Take evenly spaced permutations to get good coverage
        let step = permutations.len() / max_count;
        permutations = permutations
            .into_iter()
            .step_by(step)
            .take(max_count)
            .collect();
    }

    permutations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_permutations() {
        // 3 items = 6 permutations
        let perms = generate_permutations(3, 100);
        assert_eq!(perms.len(), 6);

        // 4 items = 24 permutations
        let perms = generate_permutations(4, 100);
        assert_eq!(perms.len(), 24);

        // Limited to 6
        let perms = generate_permutations(4, 6);
        assert_eq!(perms.len(), 6);

        // Single permutation
        let perms = generate_permutations(4, 1);
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0], vec![0, 1, 2, 3]);
    }
}
