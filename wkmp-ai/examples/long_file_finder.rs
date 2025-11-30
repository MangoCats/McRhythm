/// Scans a directory recursively for audio files exceeding a specified duration.
///
/// Usage: cargo run --example long_file_finder [--min-duration SECONDS] [--path PATH]
///
/// Default: 30 minutes (1800 seconds), scans ~/Music
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "m4a", "aac", "wma", "opus", "ape", "wv", "tta",
];

fn get_default_music_dir() -> PathBuf {
    // Windows: %USERPROFILE%\Music
    // Linux/macOS: ~/Music
    if cfg!(windows) {
        let user_profile = env::var("USERPROFILE").expect("USERPROFILE not set");
        PathBuf::from(user_profile).join("Music")
    } else {
        let home = env::var("HOME").expect("HOME not set");
        PathBuf::from(home).join("Music")
    }
}

fn main() {
    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let mut min_duration_secs = 1800; // 30 minutes default
    let mut scan_path = get_default_music_dir();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--min-duration" => {
                if i + 1 < args.len() {
                    min_duration_secs = args[i + 1].parse().unwrap_or(1800);
                    i += 2;
                } else {
                    eprintln!("Error: --min-duration requires a value");
                    std::process::exit(1);
                }
            }
            "--path" => {
                if i + 1 < args.len() {
                    scan_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --path requires a value");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    println!("Scanning: {}", scan_path.display());
    println!(
        "Minimum duration: {} minutes ({} seconds)\n",
        min_duration_secs / 60,
        min_duration_secs
    );

    let mut long_files = Vec::new();
    scan_directory(&scan_path, min_duration_secs, &mut long_files);

    // Sort by duration (longest first)
    long_files.sort_by(|a, b| b.1.cmp(&a.1));

    println!(
        "\nFound {} files longer than {} minutes:\n",
        long_files.len(),
        min_duration_secs / 60
    );
    for (path, duration_secs) in long_files {
        let hours = duration_secs / 3600;
        let minutes = (duration_secs % 3600) / 60;
        let seconds = duration_secs % 60;

        if hours > 0 {
            println!(
                "[{:02}:{:02}:{:02}] {}",
                hours,
                minutes,
                seconds,
                path.display()
            );
        } else {
            println!("[{:02}:{:02}] {}", minutes, seconds, path.display());
        }
    }
}

fn scan_directory(path: &Path, min_duration_secs: u64, results: &mut Vec<(PathBuf, u64)>) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error reading directory {}: {}", path.display(), e);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Error reading entry: {}", e);
                continue;
            }
        };

        let path = entry.path();

        if path.is_dir() {
            scan_directory(&path, min_duration_secs, results);
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if AUDIO_EXTENSIONS.contains(&ext_str.as_str()) {
                    match get_audio_duration(&path) {
                        Ok(duration_secs) => {
                            if duration_secs >= min_duration_secs {
                                results.push((path.clone(), duration_secs));
                            }
                            // Progress indicator
                            print!(".");
                            use std::io::Write;
                            std::io::stdout().flush().unwrap();
                        }
                        Err(e) => {
                            eprintln!("\nError reading {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }
}

fn get_audio_duration(path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        hint.with_extension(&ext.to_string_lossy());
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed =
        symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
    let format = probed.format;

    // Get the default track
    let track = format.default_track().ok_or("No default track found")?;

    // Calculate duration from time base and duration
    if let Some(time_base) = track.codec_params.time_base {
        if let Some(n_frames) = track.codec_params.n_frames {
            let duration_secs = time_base.calc_time(n_frames).seconds;
            return Ok(duration_secs);
        }
    }

    Err("Could not determine duration".into())
}
