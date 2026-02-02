//! CLI and server for area-proportional Venn diagrams.
//!
//! Provides:
//! - Batch training from command line
//! - WebSocket server for frontend connections
//! - Parallel scene training across different initial assignments
//! - SVG rendering of training results

mod render;
mod server;
mod trace;

use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use flate2::write::GzEncoder;
use flate2::Compression;

use clap::{Parser, Subcommand};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use apvd_core::{Model, InputSpec, TargetsMap, TieredConfig};
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
        /// Config file (JSON with inputs and targets). Alternative to -s/-t.
        #[arg(short, long, conflicts_with_all = ["shapes", "targets"])]
        config: Option<String>,

        /// Input shapes (JSON file or inline JSON). Use with -t.
        #[arg(short, long, required_unless_present_any = ["config", "resume"])]
        shapes: Option<String>,

        /// Target areas (JSON file or inline JSON). Use with -s.
        #[arg(short, long, required_unless_present_any = ["config", "resume"])]
        targets: Option<String>,

        /// Resume training from a trace file (continues from final shapes)
        #[arg(short = 'r', long, conflicts_with_all = ["config", "shapes", "targets"])]
        resume: Option<String>,

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

        /// Include tiered keyframes for efficient random seek (I-frame storage)
        /// Tier 0: 2B samples at resolution 1, Tier n: B samples at resolution 2^n
        /// Default B=1024 gives ~15:1 compression at 100k steps
        #[arg(short = 'T', long)]
        tiered: Option<Option<usize>>,

        /// Gzip compress the output (adds .gz extension if not present)
        #[arg(short = 'z', long)]
        gzip: bool,

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

    /// Trace file operations (info, convert, diff, verify, benchmark, reconstruct)
    Trace {
        #[command(subcommand)]
        command: TraceCommands,
    },
}

#[derive(Subcommand)]
enum TraceCommands {
    /// Display trace metadata and statistics
    Info {
        /// Trace file to inspect
        file: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include per-keyframe details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Verify trace integrity and reconstruction accuracy
    Verify {
        /// Trace file to verify
        file: String,

        /// Reconstruction tolerance (default: 1e-10)
        #[arg(long, default_value = "1e-10")]
        tolerance: f64,

        /// Random steps to verify (default: 100)
        #[arg(long, default_value = "100")]
        samples: usize,

        /// Verify every step (slow)
        #[arg(long)]
        exhaustive: bool,

        /// Schema only, skip reconstruction
        #[arg(long)]
        quick: bool,
    },

    /// Benchmark recomputation performance
    Benchmark {
        /// Trace file to benchmark
        file: String,

        /// Random access samples (default: 1000)
        #[arg(long, default_value = "1000")]
        samples: usize,

        /// Include sequential scan benchmark
        #[arg(long)]
        sequential: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Reconstruct and output shapes at a specific step
    Reconstruct {
        /// Trace file
        file: String,

        /// Step index to reconstruct
        step: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Render to SVG file
        #[arg(long)]
        svg: Option<String>,
    },

    /// Convert trace between tiering configurations
    Convert {
        /// Input trace file
        input: String,

        /// Output file
        #[arg(short, long)]
        output: String,

        /// Maximum BTD keyframes
        #[arg(long)]
        max_btd: Option<usize>,

        /// Interval keyframe spacing (0 to disable)
        #[arg(long)]
        interval: Option<usize>,

        /// Output as .json.gz
        #[arg(long)]
        compress: bool,

        /// Overwrite existing output
        #[arg(short, long)]
        force: bool,
    },

    /// Compare two traces
    Diff {
        /// First trace file
        file1: String,

        /// Second trace file
        file2: String,

        /// Compare at specific step
        #[arg(long)]
        step: Option<usize>,

        /// Difference tolerance (default: 1e-10)
        #[arg(long, default_value = "1e-10")]
        tolerance: f64,

        /// Compare every step (slow)
        #[arg(long)]
        all_steps: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    /// BTD (Best To Date) step indices - steps where error improved
    #[serde(skip_serializing_if = "Option::is_none")]
    btd_steps: Option<Vec<usize>>,
    /// Tiered keyframe config (if --tiered was specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    tiered_config: Option<TieredConfig>,
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
            config,
            shapes,
            targets,
            resume,
            max_steps,
            learning_rate,
            parallel,
            output,
            robust,
            quiet,
            history,
            checkpoints,
            tiered,
            gzip,
            svg,
        } => {
            if let Err(e) = run_train(config, shapes, targets, resume, max_steps, learning_rate, parallel, output, robust, quiet, history, checkpoints, tiered, gzip, svg) {
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
        Commands::Trace { command } => {
            if let Err(e) = run_trace_command(command) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn run_train(
    config_arg: Option<String>,
    shapes_arg: Option<String>,
    targets_arg: Option<String>,
    resume_arg: Option<String>,
    max_steps: usize,
    learning_rate: f64,
    parallel: usize,
    output: Option<String>,
    robust: bool,
    quiet: bool,
    include_history: bool,
    include_checkpoints: bool,
    tiered: Option<Option<usize>>,
    gzip: bool,
    svg_output: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    // Parse tiered config if specified
    let tiered_config = tiered.map(|opt_b| TieredConfig::new(opt_b));

    // Parse inputs and targets from either resume trace, config file, or separate args
    let (inputs, targets): (Vec<InputSpec>, TargetsMap<f64>) = if let Some(ref resume_path) = resume_arg {
        // Resume from trace file
        let trace_data = trace::load_trace(resume_path)?;

        // Get inputs from the trace's final shapes (converted to f64)
        let final_inputs = match &trace_data {
            trace::TraceData::Train(t) => {
                // Extract shapes from final_shapes (they're in Dual format, need to extract values)
                let shapes: Vec<apvd_core::Shape<f64>> = t.best.final_shapes.iter()
                    .map(|v| {
                        // Parse as Shape<Dual> then extract values
                        extract_shape_values(v)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // Convert to InputSpec (all coords trainable)
                shapes.iter().map(|s| {
                    let n = match s {
                        apvd_core::Shape::Circle(_) => 3,
                        apvd_core::Shape::XYRR(_) => 4,
                        apvd_core::Shape::XYRRT(_) => 5,
                        apvd_core::Shape::Polygon(p) => p.vertices.len() * 2,
                    };
                    (s.clone(), vec![true; n])
                }).collect::<Vec<InputSpec>>()
            }
            trace::TraceData::V2(t) => {
                // Use config inputs for now (V2 format has proper keyframes)
                t.config.inputs.clone()
            }
        };

        let targets = trace_data.targets().clone();
        let resume_from_step = trace_data.total_steps();

        if !quiet {
            eprintln!("Resuming from {} at step {}", resume_path, resume_from_step);
        }

        (final_inputs, targets)
    } else if let Some(config_path) = config_arg {
        // Load from unified config file
        let config_json = read_json_arg(&config_path)?;
        let test_case: TestCase = serde_json::from_str(&config_json)
            .map_err(|e| format!("Failed to parse config JSON: {}", e))?;
        (test_case.inputs, test_case.targets)
    } else {
        // Load from separate shapes and targets args
        let shapes_json = read_json_arg(shapes_arg.as_ref().ok_or("shapes argument required")?)?;
        let inputs: Vec<InputSpec> = serde_json::from_str(&shapes_json)
            .map_err(|e| format!("Failed to parse shapes JSON: {}", e))?;

        let targets_json = read_json_arg(targets_arg.as_ref().ok_or("targets argument required")?)?;
        let targets: TargetsMap<f64> = serde_json::from_str(&targets_json)
            .map_err(|e| format!("Failed to parse targets JSON: {}", e))?;

        (inputs, targets)
    };

    let num_shapes = inputs.len();
    if !quiet {
        eprintln!("Training with {} shapes, {} target regions", num_shapes, targets.len());
        eprintln!("  max_steps: {}, learning_rate: {}", max_steps, learning_rate);
        eprintln!("  optimizer: {}", if robust { "robust (Adam + clipping)" } else { "standard GD" });
        if include_history {
            eprintln!("  history: enabled (full)");
        } else if include_checkpoints {
            eprintln!("  history: enabled (checkpoints only)");
        } else if let Some(ref tc) = tiered_config {
            eprintln!("  history: enabled (tiered, B={})", tc.bucket_size);
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

            // Collect BTD (Best To Date) step indices
            let mut btd_steps = Vec::new();
            let mut min_so_far = f64::INFINITY;
            for (idx, step) in model.steps.iter().enumerate() {
                let error = step.error.v();
                if error < min_so_far {
                    min_so_far = error;
                    btd_steps.push(idx);
                }
            }

            // Build history if requested
            let history = if include_history || include_checkpoints || tiered_config.is_some() {
                // Determine which steps to include
                let include_step: Box<dyn Fn(usize) -> bool> = if include_history {
                    Box::new(|_| true) // all steps
                } else if include_checkpoints {
                    // Checkpoint steps: 0, 100, 500, 1000, best, final
                    let mut indices: std::collections::HashSet<usize> = [0, 100, 500, 1000]
                        .iter()
                        .filter(|&&i| i < model.steps.len())
                        .copied()
                        .collect();
                    indices.insert(model.min_idx); // best step
                    indices.insert(model.steps.len() - 1); // final step
                    Box::new(move |idx| indices.contains(&idx))
                } else if let Some(ref tc) = tiered_config {
                    // Tiered keyframes
                    let tc = tc.clone();
                    let final_idx = model.steps.len() - 1;
                    let min_idx = model.min_idx;
                    Box::new(move |idx| {
                        tc.is_keyframe(idx) || idx == final_idx || idx == min_idx
                    })
                } else {
                    Box::new(|_| false)
                };

                let mut min_so_far = f64::INFINITY;
                Some(
                    model.steps.iter().enumerate()
                        .filter(|(idx, _)| include_step(*idx))
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
                btd_steps: if tiered_config.is_some() || include_history { Some(btd_steps) } else { None },
                tiered_config: tiered_config.clone(),
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
        // Ensure .gz extension if gzip is enabled
        let output_path = if gzip && !output_path.ends_with(".gz") {
            format!("{}.gz", output_path)
        } else {
            output_path
        };

        if gzip {
            let file = std::fs::File::create(&output_path)?;
            let mut encoder = GzEncoder::new(file, Compression::default());
            encoder.write_all(json_output.as_bytes())?;
            encoder.finish()?;
            if !quiet {
                let uncompressed_size = json_output.len();
                let compressed_size = std::fs::metadata(&output_path)?.len() as usize;
                let ratio = uncompressed_size as f64 / compressed_size as f64;
                eprintln!("Results written to {} ({} → {} bytes, {:.1}x compression)",
                    output_path, uncompressed_size, compressed_size, ratio);
            }
        } else {
            std::fs::write(&output_path, &json_output)?;
            if !quiet {
                eprintln!("Results written to {}", output_path);
            }
        }
    } else {
        println!("{}", json_output);
    }

    // Render SVG if requested
    if let Some(svg_path) = svg_output {
        // Parse final shapes for rendering
        let shapes: Vec<apvd_core::shape::Shape<f64>> = best.final_shapes.iter()
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
    let shapes: Vec<apvd_core::shape::Shape<f64>> = shapes_json.iter()
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

/// Extract f64 shape from a JSON value that may be in Dual format.
/// Dual format has {v: number, d: [...]} for each coordinate.
fn extract_shape_values(value: &serde_json::Value) -> Result<apvd_core::Shape<f64>, Box<dyn std::error::Error>> {
    // Helper to extract f64 from either plain number or Dual {v, d} object
    fn extract_f64(v: &serde_json::Value) -> Result<f64, Box<dyn std::error::Error>> {
        if let Some(n) = v.as_f64() {
            Ok(n)
        } else if let Some(obj) = v.as_object() {
            if let Some(val) = obj.get("v") {
                val.as_f64().ok_or_else(|| "Expected f64 for 'v' field".into())
            } else {
                Err("Expected 'v' field in Dual object".into())
            }
        } else {
            Err("Expected f64 or Dual object".into())
        }
    }

    let kind = value.get("kind")
        .and_then(|k| k.as_str())
        .ok_or("Missing 'kind' field")?;

    match kind {
        "Circle" => {
            let c = value.get("c").ok_or("Missing 'c' field")?;
            let x = extract_f64(c.get("x").ok_or("Missing c.x")?)?;
            let y = extract_f64(c.get("y").ok_or("Missing c.y")?)?;
            let r = extract_f64(value.get("r").ok_or("Missing 'r' field")?)?;
            Ok(apvd_core::Shape::Circle(apvd_core::circle::Circle {
                c: apvd_core::r2::R2 { x, y },
                r,
            }))
        }
        "XYRR" => {
            let c = value.get("c").ok_or("Missing 'c' field")?;
            let r = value.get("r").ok_or("Missing 'r' field")?;
            let cx = extract_f64(c.get("x").ok_or("Missing c.x")?)?;
            let cy = extract_f64(c.get("y").ok_or("Missing c.y")?)?;
            let rx = extract_f64(r.get("x").ok_or("Missing r.x")?)?;
            let ry = extract_f64(r.get("y").ok_or("Missing r.y")?)?;
            Ok(apvd_core::Shape::XYRR(apvd_core::ellipses::xyrr::XYRR {
                c: apvd_core::r2::R2 { x: cx, y: cy },
                r: apvd_core::r2::R2 { x: rx, y: ry },
            }))
        }
        "XYRRT" => {
            let c = value.get("c").ok_or("Missing 'c' field")?;
            let r = value.get("r").ok_or("Missing 'r' field")?;
            let cx = extract_f64(c.get("x").ok_or("Missing c.x")?)?;
            let cy = extract_f64(c.get("y").ok_or("Missing c.y")?)?;
            let rx = extract_f64(r.get("x").ok_or("Missing r.x")?)?;
            let ry = extract_f64(r.get("y").ok_or("Missing r.y")?)?;
            let t = extract_f64(value.get("t").ok_or("Missing 't' field")?)?;
            Ok(apvd_core::Shape::XYRRT(apvd_core::ellipses::xyrrt::XYRRT {
                c: apvd_core::r2::R2 { x: cx, y: cy },
                r: apvd_core::r2::R2 { x: rx, y: ry },
                t,
            }))
        }
        "Polygon" => {
            let vertices = value.get("vertices")
                .and_then(|v| v.as_array())
                .ok_or("Missing 'vertices' array")?;
            let verts: Result<Vec<apvd_core::r2::R2<f64>>, Box<dyn std::error::Error>> = vertices.iter().map(|v| {
                let x = extract_f64(v.get("x").ok_or("Missing vertex x")?)?;
                let y = extract_f64(v.get("y").ok_or("Missing vertex y")?)?;
                Ok(apvd_core::r2::R2 { x, y })
            }).collect();
            Ok(apvd_core::Shape::Polygon(apvd_core::geometry::polygon::Polygon::new(verts?)))
        }
        _ => Err(format!("Unknown shape kind: {}", kind).into()),
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
                    btd_steps: None,
                    tiered_config: None,
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

fn run_trace_command(command: TraceCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        TraceCommands::Info { file, json, verbose } => {
            let trace_file = trace::load_trace(&file)?;
            let stats = trace::compute_stats(&trace_file);

            if json {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                println!("Trace: {}", file);
                println!("  Format: {}", stats.format);
                println!("  Total steps: {}", stats.total_steps);
                println!("  Min error: {:.6e} (step {})", stats.min_error, stats.min_step);
                println!();

                println!("Shapes:");
                println!("  {} shape(s)", stats.num_shapes);
                println!("  Types: {}", stats.shape_types.join(", "));
                println!("  {} variables total", stats.total_variables);
                println!();

                println!("Keyframes:");
                println!("  BTD keyframes: {}", stats.btd_keyframe_count);
                println!("  Interval keyframes: {}", stats.interval_keyframe_count);
                println!("  Total stored: {}", stats.total_keyframes);
                println!();

                if let Some(ref tiering) = stats.tiering {
                    println!("Tiering:");
                    println!("  Strategy: {}", tiering.strategy);
                    println!("  Max BTD: {}", tiering.max_btd_keyframes);
                    println!("  Interval spacing: {}", tiering.interval_spacing);
                    println!();
                }

                println!("Recomputation:");
                println!("  Max distance: {} steps", stats.max_recompute_distance);
                println!("  Avg distance: {:.1} steps", stats.avg_recompute_distance);

                if verbose {
                    println!();
                    println!("Keyframe details:");
                    for kf in trace_file.keyframes() {
                        let error_str = kf.error.map_or("(none)".to_string(), |e| format!("{:.6e}", e));
                        println!("  Step {}: error {}", kf.step_index, error_str);
                    }
                }
            }
        }

        TraceCommands::Verify { file, tolerance, samples, exhaustive, quick } => {
            let trace_file = trace::load_trace(&file)?;
            println!("Verifying {}...", file);

            let result = trace::verify_trace(&trace_file, tolerance, samples, exhaustive, quick);

            for warning in &result.warnings {
                println!("  ! {}", warning);
            }

            for error in &result.errors {
                println!("  x {}", error);
            }

            if result.samples_verified > 0 {
                println!();
                println!("Reconstruction verification ({} samples):", result.samples_verified);
                println!("  Max error: {:.6e}", result.max_reconstruction_error);
            }

            println!();
            if result.valid {
                println!("Trace verified successfully.");
            } else {
                println!("Trace verification FAILED.");
                std::process::exit(1);
            }
        }

        TraceCommands::Benchmark { file, samples, sequential, json } => {
            let trace_file = trace::load_trace(&file)?;

            if !json {
                eprintln!("Benchmarking {}...", file);
            }

            let result = trace::benchmark_trace(&trace_file, samples, sequential);

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!();
                println!("Random access ({} samples):", result.random_access.samples);
                println!("  Min: {:.2}ms (keyframe hit)", result.random_access.min_ms);
                println!("  Max: {:.2}ms", result.random_access.max_ms);
                println!("  Avg: {:.2}ms", result.random_access.avg_ms);
                println!("  P50: {:.2}ms", result.random_access.p50_ms);
                println!("  P95: {:.2}ms", result.random_access.p95_ms);
                println!("  Keyframe hits: {}", result.random_access.keyframe_hits);

                if let Some(ref seq) = result.sequential {
                    println!();
                    println!("Sequential scan (step 0 -> {}):", seq.total_steps);
                    println!("  Total: {:.2}ms", seq.total_ms);
                    println!("  Per step: {:.4}ms", seq.per_step_ms);
                }
            }
        }

        TraceCommands::Reconstruct { file, step, json, svg } => {
            let trace_file = trace::load_trace(&file)?;

            let start = std::time::Instant::now();
            let (reconstructed, kf_step) = trace::reconstruct_step(&trace_file, step)?;
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            if json {
                let shapes: Vec<serde_json::Value> = reconstructed.shapes.iter()
                    .map(|s| serde_json::to_value(s.v()).unwrap())
                    .collect();
                let output = serde_json::json!({
                    "stepIndex": step,
                    "error": reconstructed.error.v(),
                    "shapes": shapes,
                    "recomputedFrom": kf_step,
                    "recomputeSteps": step - kf_step,
                    "recomputeMs": elapsed_ms,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("Step {} (reconstructed from keyframe at {}):", step, kf_step);
                println!("  Recomputation: {} steps, {:.2}ms", step - kf_step, elapsed_ms);
                println!();
                println!("Shapes:");
                for (i, shape) in reconstructed.shapes.iter().enumerate() {
                    let s = shape.v();
                    match s {
                        apvd_core::Shape::Circle(c) => {
                            println!("  [{}] Circle: center=({:.4}, {:.4}), r={:.4}",
                                i, c.c.x, c.c.y, c.r);
                        }
                        apvd_core::Shape::XYRR(e) => {
                            println!("  [{}] XYRR: center=({:.4}, {:.4}), radii=({:.4}, {:.4})",
                                i, e.c.x, e.c.y, e.r.x, e.r.y);
                        }
                        apvd_core::Shape::XYRRT(e) => {
                            println!("  [{}] XYRRT: center=({:.4}, {:.4}), radii=({:.4}, {:.4}), theta={:.4}",
                                i, e.c.x, e.c.y, e.r.x, e.r.y, e.t);
                        }
                        apvd_core::Shape::Polygon(p) => {
                            println!("  [{}] Polygon: {} vertices", i, p.vertices.len());
                        }
                    }
                }
                println!();
                println!("Error: {:.6e}", reconstructed.error.v());
            }

            if let Some(svg_path) = svg {
                let shapes: Vec<apvd_core::Shape<f64>> = reconstructed.shapes.iter()
                    .map(|s| s.v())
                    .collect();
                let config = RenderConfig::default();
                let svg_content = render_svg(&shapes, &config);
                std::fs::write(&svg_path, &svg_content)?;
                eprintln!("SVG written to {}", svg_path);
            }
        }

        TraceCommands::Convert { input, output, max_btd: _, interval: _, compress, force } => {
            // Check if output exists
            if !force && std::path::Path::new(&output).exists() {
                return Err(format!("Output file '{}' already exists. Use --force to overwrite.", output).into());
            }

            // For now, just copy the file (with optional compression)
            // Full conversion with re-tiering would require more implementation
            eprintln!("Converting {}...", input);
            eprintln!("Note: Re-tiering not yet implemented, copying file as-is.");

            let content = std::fs::read_to_string(&input)?;

            if compress || output.ends_with(".gz") {
                let file = std::fs::File::create(&output)?;
                let mut encoder = GzEncoder::new(file, Compression::default());
                std::io::Write::write_all(&mut encoder, content.as_bytes())?;
                encoder.finish()?;
            } else {
                std::fs::write(&output, content)?;
            }

            eprintln!("Written to {}", output);
        }

        TraceCommands::Diff { file1, file2, step: _, tolerance, all_steps: _, json } => {
            let trace1 = trace::load_trace(&file1)?;
            let trace2 = trace::load_trace(&file2)?;

            if !json {
                println!("Comparing traces...");
                println!();
            }

            // Compare configs
            let inputs1 = trace1.inputs();
            let inputs2 = trace2.inputs();
            let targets1 = trace1.targets();
            let targets2 = trace2.targets();
            let lr1 = trace1.learning_rate();
            let lr2 = trace2.learning_rate();

            let configs_match = inputs1.len() == inputs2.len()
                && targets1 == targets2
                && (lr1 - lr2).abs() < 1e-10;

            // Compare steps
            let steps1 = trace1.total_steps();
            let steps2 = trace2.total_steps();
            let steps_match = steps1 == steps2;

            // Compare min error
            let error1 = trace1.min_error();
            let error2 = trace2.min_error();
            let step1 = trace1.min_step();
            let step2 = trace2.min_step();
            let error_diff = (error1 - error2).abs();
            let errors_match = error_diff < tolerance;

            if json {
                let output = serde_json::json!({
                    "file1": file1,
                    "file2": file2,
                    "configsMatch": configs_match,
                    "stepsMatch": steps_match,
                    "file1Steps": steps1,
                    "file2Steps": steps2,
                    "file1MinError": error1,
                    "file2MinError": error2,
                    "errorDiff": error_diff,
                    "errorsMatch": errors_match,
                    "tolerance": tolerance,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("Config:");
                println!("  Inputs: {} vs {} shapes {}",
                    inputs1.len(),
                    inputs2.len(),
                    if inputs1.len() == inputs2.len() { "(match)" } else { "(DIFFER)" }
                );
                println!("  Targets: {}", if targets1 == targets2 { "identical" } else { "DIFFER" });
                println!("  Learning rate: {} vs {} {}",
                    lr1, lr2,
                    if (lr1 - lr2).abs() < 1e-10 { "(identical)" } else { "(DIFFER)" }
                );
                println!();

                println!("Steps:");
                println!("  {}: {} steps", file1, steps1);
                println!("  {}: {} steps", file2, steps2);
                println!();

                println!("Error convergence:");
                println!("  {}: {:.6e} at step {}", file1, error1, step1);
                println!("  {}: {:.6e} at step {}", file2, error2, step2);
                println!("  Difference: {:.6e} {}", error_diff,
                    if errors_match { "(within tolerance)" } else { "(EXCEEDS TOLERANCE)" }
                );
            }

            if !errors_match && !json {
                std::process::exit(1);
            }
        }
    }

    Ok(())
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
