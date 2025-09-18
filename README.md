# git-radio

[![Build and Visualize](https://github.com/fschutt/git-radio/actions/workflows/visualize.yml/badge.svg)](https://github.com/fschutt/git-radio/actions/workflows/visualize.yml)

Visualize the history of a Git repository as an animated "radio frequency" heatmap. This tool analyzes your 
entire commit history to create a video showing which files and lines of code are changing the most over time.

## Introduction

`git-radio` creates a visualization where:

-   **Each vertical column represents a file** in your repository.
-   **Each pixel in a column represents a line of code** in that file.
-   **The color of a pixel shows its "hotness."** A line that changes frequently becomes more orange ("hot"), while stable lines cool down to blue ("cold").

This provides an intuitive, at-a-glance view of your codebase's evolution, helping you identify 
"hotspots" — areas with high churn that might be prone to bugs or in need of refactoring.

## Features

-   **Two Visualization Modes:**
    -   `hot-cold`: Colors lines based on change frequency within a moving time window.
    -   `committer`: Colors lines based on the last person to modify them.
-   **Adjustable Time Window:** Define the "memory" of the heatmap (e.g., how many days of history to consider for hotness).
-   **Intelligent File Tracking:** Correctly handles file renames, additions, and deletions throughout the repository's history.
-   **High Performance:** Written in Rust with parallel processing (`rayon`) for fast frame rendering.
-   **Video Output:** Generates a sequence of PNG images ready to be compiled into a video with FFmpeg.

## Prerequisites

1.  **Rust Toolchain:** Install from [rustup.rs](https://rustup.rs/).
2.  **libgit2 Dependencies:** The `git2` crate requires some system libraries.
    -   **Ubuntu/Debian:** `sudo apt-get install -y pkg-config libssl-dev libssh2-1-dev cmake`
    -   **macOS (Homebrew):** `brew install pkg-config openssl libssh2`
    -   **Windows:** No extra steps are typically needed with the `msvc` toolchain.
3.  **FFmpeg:** To create a video from the output frames.
    -   **Ubuntu/Debian:** `sudo apt-get install ffmpeg`
    -   **macOS (Homebrew):** `brew install ffmpeg`
    -   **Windows:** Download from the [official site](https://ffmpeg.org/download.html).

## Installation & Usage

1.  **Clone and build:**
    ```bash
    git clone https://github.com/fschutt/git-radio.git
    cd git-radio
    cargo build --release
    ```

2.  **Run the visualization:**

    **Basic Hot/Cold Mode:**
    ```bash
    ./target/release/git-radio \
      --repo /path/to/your/favorite/repo \
      --output ./frames_hot_cold
    ```

    **Committer Mode:**
    ```bash
    ./target/release/git-radio \
      --repo /path/to/your/favorite/repo \
      --output ./frames_committer \
      --mode committer
    ```

    **Custom Options:**
    ```bash
    ./target/release/git-radio \
      --repo /path/to/your/favorite/repo \
      --output ./custom_frames \
      --width 1920 \
      --height 1080 \
      --window-days 90
    ```

## Creating a video

After the tool finishes, you will have a directory full of PNG frames. 
Use `ffmpeg` to stitch them into a video.

```bash
ffmpeg -framerate 60 -i frames_hot_cold/frame_%06d.png -c:v libx264 -pix_fmt yuv420p -crf 18 output.mp4
```

-   `-framerate 60`: Sets the video to 60 frames per second. Adjust as needed.
-   `-i frames_hot_cold/frame_%06d.png`: Specifies the input frames.
-   `-c:v libx264 -pix_fmt yuv420p`: Standard options for high-quality, compatible video.
-   `-crf 18`: Sets the quality level (lower is better, 18 is visually lossless).
-   `output.mp4`: The name of your final video file.

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.
