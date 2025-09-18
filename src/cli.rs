// src/cli.rs

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the git repository to analyze
    #[arg(short, long)]
    pub repo: PathBuf,

    /// Directory to save the output PNG frames
    #[arg(short, long)]
    pub output: PathBuf,

    /// Width of the output images in pixels
    #[arg(long, default_value_t = 1280)]
    pub width: u32,

    /// Height of the output images in pixels
    #[arg(long, default_value_t = 720)]
    pub height: u32,

    /// The size of the moving window for hotness calculation, in days
    #[arg(long, default_value_t = 30)]
    pub window_days: u64,

    /// Visualization mode
    #[arg(long, value_enum, default_value_t = Mode::HotCold)]
    pub mode: Mode,
}

#[derive(clap::ValueEnum, Clone, Debug, Copy)]
pub enum Mode {
    /// Color lines by change frequency (blue=cold, orange=hot)
    HotCold,
    /// Color lines by the last committer within the window
    Committer,
}
