//! CLI and server for area-proportional Venn diagrams.
//!
//! Provides:
//! - Batch training from command line
//! - WebSocket server for frontend connections
//! - Parallel scene training across different initial assignments

use clap::{Parser, Subcommand};

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
        /// Input shapes (JSON format)
        #[arg(short, long)]
        shapes: String,

        /// Target areas (JSON format)
        #[arg(short, long)]
        targets: String,

        /// Maximum training steps
        #[arg(short, long, default_value = "1000")]
        max_steps: usize,

        /// Learning rate
        #[arg(short, long, default_value = "0.05")]
        learning_rate: f64,

        /// Number of parallel scene variants to train
        #[arg(short, long, default_value = "1")]
        parallel: usize,

        /// Output file for results (JSON)
        #[arg(short, long)]
        output: Option<String>,
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
        } => {
            println!("Training mode (not yet implemented)");
            println!("  shapes: {}", shapes);
            println!("  targets: {}", targets);
            println!("  max_steps: {}", max_steps);
            println!("  learning_rate: {}", learning_rate);
            println!("  parallel: {}", parallel);
            println!("  output: {:?}", output);
            // TODO: Implement training
        }
        Commands::Serve { port, parallel } => {
            println!("Server mode (not yet implemented)");
            println!("  port: {}", port);
            println!("  parallel: {}", parallel);
            // TODO: Implement WebSocket server
        }
    }
}
