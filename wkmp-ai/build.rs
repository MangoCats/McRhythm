// Build script to capture build timestamp

fn main() {
    // Capture build timestamp in ISO8601 format with timezone
    let now = chrono::Local::now();
    let build_timestamp = now.to_rfc3339(); // ISO8601 with timezone: 2025-10-31T13:45:22-05:00

    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);
    println!("cargo:rerun-if-changed=build.rs");
}
