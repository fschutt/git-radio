// src/renderer.rs

use crate::cli::{Args, Mode};
use crate::model::*;
use chrono::{Duration, Utc, TimeZone};
use image::{Rgb, RgbImage};
use indicatif::{ParallelProgressIterator, ProgressBar};
use palette::{Gradient, Lch, Srgb};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::fs;

pub fn render_frames(analysis: &AnalysisResult, args: &Args) {
    fs::create_dir_all(&args.output).expect("Failed to create output directory");

    let window_seconds = Duration::days(args.window_days as i64).num_seconds();
    let total_minutes = (analysis.end_time - analysis.start_time) / 60;
    
    let bar = ProgressBar::new(total_minutes as u64);
    bar.set_message("Rendering frames");

    // Pre-generate committer colors for consistency
    let committer_colors = generate_committer_colors(analysis.committers.len());

    // Create a BTreeMap of commit time -> state for quick lookups
    // This simplifies finding the active state for any given minute.
    let mut commit_times = BTreeMap::new();
    for &(_, ts) in &analysis.commits {
        commit_times.insert(ts, ());
    }

    (0..=total_minutes).into_par_iter().progress_with(bar).for_each(|i| {
        let current_time = analysis.start_time + i * 60;
        let frame_path = args.output.join(format!("frame_{:06}.png", i));

        // Find the most recent commit time that is <= current_time
        let active_commit_time = commit_times.range(..=current_time).next_back().map_or(analysis.start_time, |(&ts, _)| ts);

        let mut image = RgbImage::new(args.width, args.height);
        render_frame(
            &mut image,
            current_time,
            active_commit_time,
            window_seconds,
            analysis,
            args,
            &committer_colors,
        );
        image.save(&frame_path).expect("Failed to save frame");
    });
}

fn render_frame(
    image: &mut RgbImage,
    current_time: i64,
    active_commit_time: i64,
    window_seconds: i64,
    analysis: &AnalysisResult,
    args: &Args,
    committer_colors: &[Rgb<u8>],
) {
    // 1. Determine which files are "alive" at this time
    let active_files: Vec<&FileInfo> = analysis.files.iter().filter(|f| {
        f.birth_time <= active_commit_time && f.death_time.map_or(true, |d| d > active_commit_time)
    }).collect();

    if active_files.is_empty() { return; }

    // 2. Determine the layout
    let max_lines = active_files.iter().map(|f| {
        // Get the line count at the active commit time
        f.line_counts.range(..=active_commit_time).next_back().map_or(0, |(_, &count)| count)
    }).max().unwrap_or(1) as f32;
    
    let num_files = active_files.len() as f32;
    let file_width = args.width as f32 / num_files;
    let bg_color = Rgb([8, 8, 12]);

    // Pre-calculate heat/committer for each active file/line to avoid redundant lookups
    let mut line_data_cache = HashMap::new();
    for (file_idx, file_info) in active_files.iter().enumerate() {
        let line_count = file_info.line_counts.range(..=active_commit_time).next_back().map_or(0, |(_, &c)| c);
        for line_num in 0..line_count {
            if let Some(history) = analysis.changes.get(&(file_info.id, line_num + 1)) {
                 let window_start = current_time - window_seconds;
                 
                 match args.mode {
                    Mode::HotCold => {
                        let heat = history.iter().filter(|c| c.timestamp >= window_start && c.timestamp <= current_time).count();
                        line_data_cache.insert((file_idx, line_num), (heat, 0)); // 0 for committer_id is unused
                    }
                    Mode::Committer => {
                        let last_committer = history.iter()
                            .filter(|c| c.timestamp <= current_time)
                            .last()
                            .map(|c| c.committer_id);
                        if let Some(id) = last_committer {
                            line_data_cache.insert((file_idx, line_num), (0, id)); // 0 for heat is unused
                        }
                    }
                 }
            }
        }
    }


    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let file_idx = (x as f32 / file_width).floor() as usize;
        
        if file_idx >= active_files.len() {
             *pixel = bg_color;
             continue;
        }

        let line_num = (y as f32 / args.height as f32 * max_lines).floor() as usize;

        if let Some(&(heat, committer_id)) = line_data_cache.get(&(file_idx, line_num)) {
            *pixel = match args.mode {
                Mode::HotCold => heat_to_color(heat),
                Mode::Committer => committer_colors.get(committer_id).unwrap_or(&bg_color).clone(),
            };
        } else {
            *pixel = bg_color;
        }
    }
}

// Blue-to-Orange color gradient for hotness
fn heat_to_color(heat: usize) -> Rgb<u8> {
    let gradient = Gradient::new(vec![
        Lch::new(20.0, 30.0, 250.0), // Dark Blue
        Lch::new(40.0, 40.0, 260.0), // Blue
        Lch::new(95.0, 35.0, 90.0),  // Light Yellow
        Lch::new(75.0, 80.0, 50.0),  // Orange
        Lch::new(65.0, 100.0, 30.0), // Red-Orange
    ]);

    // Clamp heat for a reasonable visual range
    let heat_float = (heat as f32 / 10.0).min(1.0);
    let color = gradient.get(heat_float);
    let (r, g, b) = Srgb::from_linear(color.into()).into_components();
    Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8])
}

fn generate_committer_colors(num_committers: usize) -> Vec<Rgb<u8>> {
    let mut rng = StdRng::seed_from_u64(42); // Seed for deterministic colors
    (0..num_committers)
        .map(|_| {
            let hue = rng.gen_range(0.0..360.0);
            let color = Lch::new(70.0, 80.0, hue); // Bright, saturated colors
            let (r, g, b) = Srgb::from_linear(color.into()).into_components();
            Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8])
        })
        .collect()
}
