use std::path::PathBuf;
use wkmp_ai::workflow::boundary_detector::detect_boundaries_with_audio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = PathBuf::from(r"C:\Users\Mango Cat\Music\Z.Z. Top\ZZTopsFirstAlbum.mp3");

    println!("Detecting boundaries for: {}", file_path.display());
    println!();

    let result = detect_boundaries_with_audio(&file_path).await?;

    println!("Total passages: {}", result.boundaries.len());
    println!("Sample rate: {}", result.sample_rate);
    println!("Channels: {}", result.num_channels);
    println!();

    let mut total_duration = 0.0;
    const TICKS_PER_SECOND: f64 = 28_224_000.0; // SPEC017 tick rate

    for (i, boundary) in result.boundaries.iter().enumerate() {
        let start_sec = boundary.start_time as f64 / TICKS_PER_SECOND;
        let end_sec = boundary.end_time as f64 / TICKS_PER_SECOND;
        let duration_sec = end_sec - start_sec;
        total_duration += duration_sec;

        println!("Passage {:2}: {:7.2}s - {:7.2}s (duration: {:6.2}s)",
            i + 1, start_sec, end_sec, duration_sec);
    }

    println!();
    println!("Total duration: {:.2}s ({:.2} minutes)", total_duration, total_duration / 60.0);

    Ok(())
}
