// src/main.rs

mod analyzer;
mod cli;
mod model;
mod renderer;

use clap::Parser;
use chrono::TimeZone;
use cli::Args;
use std::time::Instant;

fn main() {
    let args = Args::parse();
    let start_time = Instant::now();

    match analyzer::analyze(&args.repo) {
        Ok(analysis_result) => {
            println!("Analysis finished in {:.2?}. Found {} files, {} committers.", start_time.elapsed(), analysis_result.files.len(), analysis_result.committers.len());
            println!("Repository history spans from {} to {}.",
                chrono::Utc.timestamp_opt(analysis_result.start_time, 0).unwrap().to_rfc2822(),
                chrono::Utc.timestamp_opt(analysis_result.end_time, 0).unwrap().to_rfc2822()
            );
            
            println!("Starting frame rendering...");
            let render_start = Instant::now();
            renderer::render_frames(&analysis_result, &args);
            println!("Rendering finished in {:.2?}.", render_start.elapsed());
        }
        Err(e) => {
            eprintln!("Error analyzing repository: {}", e);
        }
    }
    
    println!("Total time: {:.2?}", start_time.elapsed());
}
