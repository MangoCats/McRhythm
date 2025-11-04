//! Build script for wkmp-ai
//!
//! Captures build identification information at compile time:
//! - Git commit hash (short form)
//! - Build timestamp
//! - Build profile (debug/release)
//!
//! **[ARCH-INIT-004]** Build identification requirement

use std::process::Command;

fn main() {
    // Capture git commit hash (short form, 8 characters)
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Capture build timestamp in ISO 8601 format with local timezone
    // Format: YYYY-MM-DDTHH:MM:SS±HH:MM (e.g., 2025-10-26T14:30:45-05:00)
    let build_timestamp = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);

    // Determine build profile
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());

    // Make values available to the binary via environment variables
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);

    // Force rebuild on every build to update timestamp and git hash
    // By not specifying any rerun-if-changed directives, Cargo will rerun this script every time
}
