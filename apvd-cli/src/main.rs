//! CLI and server for area-proportional Venn diagrams.
//!
//! Provides:
//! - Batch training from command line
//! - WebSocket server for frontend connections
//! - Parallel scene training across different initial assignments
//! - SVG rendering of training results

mod render;
mod server;

use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use clap::{Parser, Subcommand};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use apvd_core::{Model, InputSpec, TargetsMap};
use render::{render_svg, RenderConfig};

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

        /// Include full step history in output (for trace inspection/time-travel)
        #[arg(short = 'H', long)]
        history: bool,

        /// Include sparse checkpoints only (steps 0, 100, 500, 1000, best, final)
        /// Smaller output but sufficient for reproducibility verification
        #[arg(short = 'C', long)]
        checkpoints: bool,

        /// Render SVG of final result to this file
        #[arg(long)]
        svg: Option<String>,
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

    /// Render shapes to SVG
    Render {
        /// Input file (training output JSON or trace JSON)
        #[arg(short, long)]
        input: String,

        /// Output SVG file
        #[arg(short, long)]
        output: String,

        /// Step index to render (default: final step, or use "best" for min error step)
        #[arg(long)]
        step: Option<String>,

        /// Which trace to render (for multi-trace sessions)
        #[arg(long, default_value = "0")]
        trace: usize,

        /// SVG width in pixels
        #[arg(long, default_value = "800")]
        width: f64,

        /// SVG height in pixels
        #[arg(long, default_value = "600")]
        height: f64,

        /// Hide shape labels
        #[arg(long)]
        no_labels: bool,
    },

    /// Run test cases from JSON files
    Test {
        /// Test case files (JSON). Can specify multiple files or use glob patterns.
        #[arg(required = true)]
        files: Vec<String>,

        /// Maximum training steps
        #[arg(short, long, default_value = "1000")]
        max_steps: usize,

        /// Save traces for failed tests to this directory
        #[arg(long)]
        save_failed: Option<String>,

        /// Strict mode: verify checkpoint errors exactly match golden trace
        #[arg(short = 'S', long)]
        strict: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Benchmark test cases
    Bench {
        /// Test case files (JSON). Can specify multiple files or use glob patterns.
        #[arg(required = true)]
        files: Vec<String>,

        /// Maximum training steps
        #[arg(short, long, default_value = "1000")]
        max_steps: usize,

        /// Number of iterations per test case
        #[arg(short, long, default_value = "3")]
        iterations: usize,
    },
}

/// A test case definition loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestCase {
    name: String,
    #[serde(default)]
    description: Option<String>,
    inputs: Vec<InputSpec>,
    targets: TargetsMap<f64>,
    #[serde(default)]
    expected: Option<ExpectedResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExpectedResult {
    /// Maximum acceptable error
    #[serde(default)]
    max_error: Option<f64>,
    /// Path to golden trace file (relative to test case file)
    #[serde(default)]
    golden_trace: Option<String>,
}

/// A single step in the training history (for trace output)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraceStep {
    /// Step index
    step_idx: usize,
    /// Error at this step
    error: f64,
    /// Shapes at this step
    shapes: Vec<serde_json::Value>,
    /// Whether this was a "best to date" step
    #[serde(skip_serializing_if = "Option::is_none")]
    is_best: Option<bool>,
}

/// A training trace (single permutation run)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrainingTrace {
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
    /// Full step history (if --history was specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    history: Option<Vec<TraceStep>>,
}

/// Combined output for all training variants (a "session")
#[derive(Debug, Serialize, Deserialize)]
struct TrainingSession {
    /// Input shapes specification
    inputs: Vec<serde_json::Value>,
    /// Target areas
    targets: TargetsMap<f64>,
    /// Best trace (lowest final error)
    best: TrainingTrace,
    /// All traces, sorted by final error
    traces: Vec<TrainingTrace>,
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
            history,
            checkpoints,
            svg,
        } => {
            if let Err(e) = run_train(shapes, targets, max_steps, learning_rate, parallel, output, robust, quiet, history, checkpoints, svg) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Serve { port, parallel } => {
            let config = server::ServerConfig { parallel };
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            if let Err(e) = rt.block_on(server::run_server(port, config)) {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Render {
            input,
            output,
            step,
            trace,
            width,
            height,
            no_labels,
        } => {
            if let Err(e) = run_render(input, output, step, trace, width, height, no_labels) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Test {
            files,
            max_steps,
            save_failed,
            strict,
            verbose,
        } => {
            if let Err(e) = run_tests(files, max_steps, save_failed, strict, verbose) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Bench {
            files,
            max_steps,
            iterations,
        } => {
            if let Err(e) = run_bench(files, max_steps, iterations) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
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
    include_history: bool,
    include_checkpoints: bool,
    svg_output: Option<String>,
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
        if include_history {
            eprintln!("  history: enabled (full)");
        } else if include_checkpoints {
            eprintln!("  history: enabled (checkpoints only)");
        }
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
    let traces: Vec<TrainingTrace> = permutations
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

            // Build history if requested
            let history = if include_history || include_checkpoints {
                // Checkpoint steps: 0, 100, 500, 1000, best, final
                let checkpoint_indices: std::collections::HashSet<usize> = if include_checkpoints {
                    let mut indices: std::collections::HashSet<usize> = [0, 100, 500, 1000]
                        .iter()
                        .filter(|&&i| i < model.steps.len())
                        .copied()
                        .collect();
                    indices.insert(model.min_idx); // best step
                    indices.insert(model.steps.len() - 1); // final step
                    indices
                } else {
                    (0..model.steps.len()).collect() // all steps
                };

                let mut min_so_far = f64::INFINITY;
                Some(
                    model.steps.iter().enumerate()
                        .filter(|(idx, _)| checkpoint_indices.contains(idx))
                        .map(|(idx, step)| {
                            let error = step.error.v();
                            let is_best = if error < min_so_far {
                                min_so_far = error;
                                Some(true)
                            } else {
                                None
                            };
                            TraceStep {
                                step_idx: idx,
                                error,
                                shapes: step.shapes.iter().map(|s| serde_json::to_value(s).unwrap()).collect(),
                                is_best,
                            }
                        }).collect()
                )
            } else {
                None
            };

            let trace = TrainingTrace {
                variant_id,
                permutation: permutation.clone(),
                final_error: final_step.error.v(),
                min_error: model.min_error,
                min_step: model.min_idx,
                total_steps: model.steps.len(),
                training_time_ms,
                final_shapes,
                history,
            };

            // Update progress
            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
            if !quiet && num_variants > 1 {
                eprint!("\rCompleted {}/{} variants", done, num_variants);
                io::stderr().flush().ok();
            }

            trace
        })
        .collect();

    if !quiet && num_variants > 1 {
        eprintln!(); // Newline after progress
    }

    // Sort by final error and find best
    let mut sorted_traces = traces;
    sorted_traces.sort_by(|a, b| a.final_error.partial_cmp(&b.final_error).unwrap());

    let best = sorted_traces[0].clone();
    let total_time_ms = start_time.elapsed().as_millis() as u64;

    if !quiet {
        eprintln!("\nBest result: variant {} with error {:.6e}", best.variant_id, best.final_error);
        eprintln!("  permutation: {:?}", best.permutation);
        eprintln!("  min_error: {:.6e} at step {}", best.min_error, best.min_step);
        eprintln!("  total time: {}ms", total_time_ms);
    }

    // Store inputs as JSON values for the session
    let inputs_json: Vec<serde_json::Value> = inputs.iter()
        .map(|i| serde_json::to_value(i).unwrap())
        .collect();

    let session = TrainingSession {
        inputs: inputs_json,
        targets: targets.clone(),
        best: best.clone(),
        traces: sorted_traces,
        total_time_ms,
    };

    // Output results
    let json_output = serde_json::to_string_pretty(&session)?;
    if let Some(output_path) = output {
        std::fs::write(&output_path, &json_output)?;
        if !quiet {
            eprintln!("Results written to {}", output_path);
        }
    } else {
        println!("{}", json_output);
    }

    // Render SVG if requested
    if let Some(svg_path) = svg_output {
        // Parse final shapes back to Shape<D> for rendering
        let shapes: Vec<apvd_core::shape::Shape<apvd_core::D>> = best.final_shapes.iter()
            .map(|v| serde_json::from_value(v.clone()).expect("Failed to parse shape"))
            .collect();

        let config = RenderConfig::default();
        let svg = render_svg(&shapes, &config);
        std::fs::write(&svg_path, &svg)?;
        if !quiet {
            eprintln!("SVG written to {}", svg_path);
        }
    }

    Ok(())
}

fn run_render(
    input: String,
    output: String,
    step: Option<String>,
    trace_idx: usize,
    width: f64,
    height: f64,
    no_labels: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the session/trace file
    let json_content = std::fs::read_to_string(&input)
        .map_err(|e| format!("Failed to read '{}': {}", input, e))?;

    let session: TrainingSession = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse session JSON: {}", e))?;

    // Get the requested trace
    if trace_idx >= session.traces.len() {
        return Err(format!(
            "Trace index {} out of range (session has {} traces)",
            trace_idx, session.traces.len()
        ).into());
    }
    let trace = &session.traces[trace_idx];

    // Determine which shapes to render
    let shapes_json: &[serde_json::Value] = match &step {
        None => {
            // Default: final step
            &trace.final_shapes
        }
        Some(s) if s == "best" => {
            // Best (minimum error) step
            if let Some(history) = &trace.history {
                &history[trace.min_step].shapes
            } else {
                return Err("Cannot use --step=best without history (use --history when training)".into());
            }
        }
        Some(s) => {
            // Specific step index
            let step_idx: usize = s.parse()
                .map_err(|_| format!("Invalid step '{}': expected number or 'best'", s))?;
            if let Some(history) = &trace.history {
                if step_idx >= history.len() {
                    return Err(format!(
                        "Step index {} out of range (trace has {} steps)",
                        step_idx, history.len()
                    ).into());
                }
                &history[step_idx].shapes
            } else {
                return Err("Cannot specify step without history (use --history when training)".into());
            }
        }
    };

    // Parse shapes
    let shapes: Vec<apvd_core::shape::Shape<apvd_core::D>> = shapes_json.iter()
        .map(|v| serde_json::from_value(v.clone()).expect("Failed to parse shape"))
        .collect();

    // Render
    let config = RenderConfig {
        width,
        height,
        show_labels: !no_labels,
        ..Default::default()
    };
    let svg = render_svg(&shapes, &config);
    std::fs::write(&output, &svg)?;

    eprintln!("SVG written to {}", output);
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

/// Run test cases from JSON files
fn run_tests(
    files: Vec<String>,
    max_steps: usize,
    save_failed: Option<String>,
    strict: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Expand glob patterns
    let test_files = expand_file_patterns(&files)?;

    if test_files.is_empty() {
        return Err("No test files found".into());
    }

    eprintln!("Running {} test case(s){}...\n",
        test_files.len(),
        if strict { " (strict mode)" } else { "" }
    );

    let mut passed = 0;
    let mut failed = 0;
    let mut results: Vec<(String, bool, f64, Option<f64>)> = Vec::new();

    for file_path in &test_files {
        // Load test case
        let json_content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read '{}': {}", file_path, e))?;

        let test_case: TestCase = serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse '{}': {}", file_path, e))?;

        // Load golden trace if strict mode and trace exists
        let golden_trace: Option<TrainingSession> = if strict {
            if let Some(ref expected) = test_case.expected {
                if let Some(ref trace_path) = expected.golden_trace {
                    let trace_content = std::fs::read_to_string(trace_path)
                        .map_err(|e| format!("Failed to read golden trace '{}': {}", trace_path, e))?;
                    Some(serde_json::from_str(&trace_content)
                        .map_err(|e| format!("Failed to parse golden trace '{}': {}", trace_path, e))?)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if verbose {
            eprintln!("Running: {} ({} shapes, {} targets{})",
                test_case.name,
                test_case.inputs.len(),
                test_case.targets.len(),
                if golden_trace.is_some() { ", with golden trace" } else { "" }
            );
        }

        // Train
        let start = Instant::now();
        let mut model = Model::new(test_case.inputs.clone(), test_case.targets.clone())
            .map_err(|e| format!("Failed to create model for '{}': {}", test_case.name, e))?;

        model.train(0.05, max_steps)
            .map_err(|e| format!("Training failed for '{}': {}", test_case.name, e))?;

        let elapsed_ms = start.elapsed().as_millis();
        let final_error = model.min_error;

        // Check against expected max error
        let max_error = test_case.expected.as_ref().and_then(|e| e.max_error);
        let mut test_passed = max_error.map_or(true, |me| final_error <= me);

        // Strict mode: verify against golden trace checkpoints
        let mut strict_failures: Vec<String> = Vec::new();
        if strict {
            if let Some(ref golden) = golden_trace {
                if let Some(ref golden_history) = golden.traces[0].history {
                    for checkpoint in golden_history {
                        let step_idx = checkpoint.step_idx;
                        if step_idx < model.steps.len() {
                            let actual_error = model.steps[step_idx].error.v();
                            let expected_error = checkpoint.error;
                            // Allow tiny floating-point tolerance
                            let diff = (actual_error - expected_error).abs();
                            let rel_diff = if expected_error != 0.0 {
                                diff / expected_error.abs()
                            } else {
                                diff
                            };
                            if rel_diff > 1e-12 && diff > 1e-15 {
                                strict_failures.push(format!(
                                    "step {}: expected {:.15e}, got {:.15e} (diff: {:.6e})",
                                    step_idx, expected_error, actual_error, diff
                                ));
                            }
                        }
                    }
                }
            }
            if !strict_failures.is_empty() {
                test_passed = false;
            }
        }

        if test_passed {
            passed += 1;
            eprintln!("  PASS: {} (error: {:.6e}, {}ms)", test_case.name, final_error, elapsed_ms);
        } else {
            failed += 1;
            if !strict_failures.is_empty() {
                eprintln!("  FAIL: {} (strict verification failed, {}ms)", test_case.name, elapsed_ms);
                for failure in &strict_failures {
                    eprintln!("         {}", failure);
                }
            } else {
                eprintln!("  FAIL: {} (error: {:.6e}, expected <= {:.6e}, {}ms)",
                    test_case.name, final_error, max_error.unwrap(), elapsed_ms);
            }

            // Save failed trace if requested
            if let Some(ref save_dir) = save_failed {
                std::fs::create_dir_all(save_dir)?;
                let trace_path = format!("{}/{}-failed.json", save_dir, test_case.name);

                let final_step = model.steps.last().unwrap();
                let final_shapes: Vec<serde_json::Value> = final_step.shapes.iter()
                    .map(|s| serde_json::to_value(s).unwrap())
                    .collect();

                let trace = TrainingTrace {
                    variant_id: 0,
                    permutation: (0..test_case.inputs.len()).collect(),
                    final_error,
                    min_error: model.min_error,
                    min_step: model.min_idx,
                    total_steps: model.steps.len(),
                    training_time_ms: elapsed_ms as u64,
                    final_shapes,
                    history: None,
                };

                let inputs_json: Vec<serde_json::Value> = test_case.inputs.iter()
                    .map(|i| serde_json::to_value(i).unwrap())
                    .collect();

                let session = TrainingSession {
                    inputs: inputs_json,
                    targets: test_case.targets.clone(),
                    best: trace.clone(),
                    traces: vec![trace],
                    total_time_ms: elapsed_ms as u64,
                };

                let json_output = serde_json::to_string_pretty(&session)?;
                std::fs::write(&trace_path, &json_output)?;
                eprintln!("         Trace saved to: {}", trace_path);
            }
        }

        results.push((test_case.name, test_passed, final_error, max_error));
    }

    eprintln!("\n{} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Run benchmarks on test cases
fn run_bench(
    files: Vec<String>,
    max_steps: usize,
    iterations: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Expand glob patterns
    let test_files = expand_file_patterns(&files)?;

    if test_files.is_empty() {
        return Err("No test files found".into());
    }

    eprintln!("Benchmarking {} test case(s) with {} iterations each...\n", test_files.len(), iterations);

    // Print header
    eprintln!("{:<25} {:>10} {:>10} {:>10} {:>12} {:>10}",
        "Name", "Shapes", "Targets", "Steps", "Time (ms)", "Error");
    eprintln!("{}", "-".repeat(80));

    for file_path in &test_files {
        // Load test case
        let json_content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read '{}': {}", file_path, e))?;

        let test_case: TestCase = serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse '{}': {}", file_path, e))?;

        let mut times: Vec<u128> = Vec::with_capacity(iterations);
        let mut errors: Vec<f64> = Vec::with_capacity(iterations);
        let mut steps: Vec<usize> = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            let mut model = Model::new(test_case.inputs.clone(), test_case.targets.clone())?;
            model.train(0.05, max_steps)?;
            times.push(start.elapsed().as_millis());
            errors.push(model.min_error);
            steps.push(model.min_idx);
        }

        // Compute statistics
        let avg_time: f64 = times.iter().map(|&t| t as f64).sum::<f64>() / iterations as f64;
        let avg_error: f64 = errors.iter().sum::<f64>() / iterations as f64;
        let avg_steps: f64 = steps.iter().map(|&s| s as f64).sum::<f64>() / iterations as f64;

        eprintln!("{:<25} {:>10} {:>10} {:>10.0} {:>12.1} {:>10.2e}",
            test_case.name,
            test_case.inputs.len(),
            test_case.targets.len(),
            avg_steps,
            avg_time,
            avg_error
        );
    }

    Ok(())
}

/// Expand file patterns (globs) to a list of file paths
fn expand_file_patterns(patterns: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    for pattern in patterns {
        // Check if it's a glob pattern
        if pattern.contains('*') || pattern.contains('?') {
            // Use glob crate would be better, but for now just try direct paths
            // For simple cases, check if it looks like testcases/*.json
            if pattern.ends_with("*.json") {
                let dir = pattern.trim_end_matches("*.json");
                let dir = if dir.is_empty() { "." } else { dir.trim_end_matches('/') };

                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map_or(false, |ext| ext == "json") {
                            files.push(path.to_string_lossy().to_string());
                        }
                    }
                }
            } else {
                // Fall back to treating as literal path
                files.push(pattern.clone());
            }
        } else {
            files.push(pattern.clone());
        }
    }

    files.sort();
    Ok(files)
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
