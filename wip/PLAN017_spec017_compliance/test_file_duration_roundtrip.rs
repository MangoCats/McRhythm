// TC-U-002: File Duration Roundtrip Test
// Tests conversion accuracy for file durations (seconds → ticks → seconds)

#[cfg(test)]
mod file_duration_roundtrip_tests {
    use wkmp_common::timing::{seconds_to_ticks, ticks_to_seconds};

    #[test]
    fn test_duration_roundtrip_accuracy() {
        let test_cases = vec![
            (0.5, 14_112_000_i64),
            (5.0, 141_120_000_i64),
            (180.5, 5_094_432_000_i64),
            (0.000001, 28_i64),  // 1 microsecond
            (3600.0, 101_606_400_000_i64),  // 1 hour
        ];

        println!("\nTC-U-002: File Duration Roundtrip Test");
        println!("{}", "=".repeat(60));

        let mut all_passed = true;

        for (input_seconds, expected_ticks) in test_cases {
            // Convert seconds → ticks
            let ticks = seconds_to_ticks(input_seconds);

            if ticks != expected_ticks {
                println!("❌ FAIL: seconds_to_ticks({}) = {}, expected {}",
                    input_seconds, ticks, expected_ticks);
                all_passed = false;
                continue;
            }

            // Convert ticks → seconds (roundtrip)
            let roundtrip_seconds = ticks_to_seconds(ticks);
            let error = (roundtrip_seconds - input_seconds).abs();

            // For most values, error should be negligible
            let max_error = if input_seconds < 0.00001 {
                1e-6  // Relaxed for very small values
            } else {
                1e-9  // Strict for normal values
            };

            if error >= max_error {
                println!("❌ FAIL: Roundtrip error for {} seconds: {} (max: {})",
                    input_seconds, error, max_error);
                all_passed = false;
            } else {
                println!("✅ PASS: {}s → {} ticks → {}s (error: {:.2e})",
                    input_seconds, ticks, roundtrip_seconds, error);
            }
        }

        println!("{}", "=".repeat(60));
        assert!(all_passed, "Some roundtrip tests failed");
        println!("\n✅ TC-U-002: PASS - All roundtrip tests passed\n");
    }

    #[test]
    fn test_zero_duration() {
        println!("\nTC-U-002 (Zero Duration): Testing zero duration handling");

        let ticks = seconds_to_ticks(0.0);
        assert_eq!(ticks, 0);

        let seconds = ticks_to_seconds(0);
        assert_eq!(seconds, 0.0);

        println!("✅ PASS: Zero duration handled correctly\n");
    }

    #[test]
    fn test_negative_duration_handling() {
        println!("\nTC-U-002 (Negative Duration): Testing negative duration handling");

        // Negative durations should not occur in practice,
        // but conversion should handle them mathematically
        let ticks = seconds_to_ticks(-5.0);
        assert_eq!(ticks, -141_120_000);

        let seconds = ticks_to_seconds(-141_120_000);
        assert_eq!(seconds, -5.0);

        println!("✅ PASS: Negative duration handled correctly\n");
    }
}
