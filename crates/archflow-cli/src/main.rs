use clap::{Parser, Subcommand};
use std::fs;
use std::process;

use archflow_core::resolver::resolve_ir;

#[derive(Parser)]
#[command(name = "archflow", about = "Archflow - Diagram as Code")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a diagram file (.archflow DSL or .json IR) to SVG
    Render {
        /// Input file (.archflow or .json)
        input: String,
        /// Output SVG file
        #[arg(short, long, default_value = "output.svg")]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Render { input, output } => {
            let content = match fs::read_to_string(&input) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {}: {}", input, e);
                    process::exit(1);
                }
            };

            // Parse to IR
            let mut ir = if input.ends_with(".archflow") {
                match archflow_core::parse_dsl(&content) {
                    Ok(ir) => ir,
                    Err(e) => {
                        eprintln!("Parse error: {}", e);
                        process::exit(1);
                    }
                }
            } else {
                match serde_json::from_str(&content) {
                    Ok(ir) => ir,
                    Err(e) => {
                        eprintln!("JSON error: {}", e);
                        process::exit(1);
                    }
                }
            };

            // Resolve icons and styles (per-provider source chains built automatically)
            resolve_ir(&mut ir, &[]);

            // Render
            let render_result = serde_json::to_string(&ir)
                .map_err(|e| e.to_string())
                .and_then(|json| archflow_core::render_svg(&json).map_err(|e| e.to_string()));
            match render_result {
                Ok(svg) => {
                    if let Err(e) = fs::write(&output, &svg) {
                        eprintln!("Error writing {}: {}", output, e);
                        process::exit(1);
                    }
                    println!("Rendered {} -> {}", input, output);
                }
                Err(e) => {
                    eprintln!("Render error: {}", e);
                    process::exit(1);
                }
            }
        }
    }
}
