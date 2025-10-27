//! Report generation and formatting
//!
//! **Purpose:** Generate CLI output and JSON export for tuning results.
//!
//! **Traceability:** TUNE-OUT-010, TUNE-OUT-030, TUNE-OUT-040

use crate::tuning::{CurvePoint, Recommendations, SystemInfo, TestResult};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Complete tuning session report
///
/// Contains all data from a tuning run: system info, test results, curve, and recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningReport {
    /// Session metadata
    pub session: SessionInfo,

    /// System information
    pub system_info: SystemInfo,

    /// All test results (raw data)
    pub test_results: Vec<TestResult>,

    /// Curve data points
    pub curve_data: Vec<CurvePoint>,

    /// Final recommendations
    pub recommendations: Option<Recommendations>,
}

/// Tuning session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session start timestamp (ISO 8601)
    pub timestamp: String,

    /// Total tuning duration in seconds
    pub duration_seconds: u64,

    /// Tuning version/format
    pub version: String,
}

impl TuningReport {
    /// Create a new tuning report
    pub fn new(
        system_info: SystemInfo,
        test_results: Vec<TestResult>,
        curve_data: Vec<CurvePoint>,
        recommendations: Option<Recommendations>,
        duration_seconds: u64,
    ) -> Self {
        Self {
            session: SessionInfo {
                timestamp: chrono::Utc::now().to_rfc3339(),
                duration_seconds,
                version: "1.0".to_string(),
            },
            system_info,
            test_results,
            curve_data,
            recommendations,
        }
    }

    /// Export report to JSON file
    ///
    /// **Traceability:** TUNE-OUT-030, TUNE-OUT-040
    pub fn export_json<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Import report from JSON file
    pub fn import_json<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let report: TuningReport = serde_json::from_reader(file)?;
        Ok(report)
    }
}

/// CLI formatter for tuning results
pub struct CliFormatter;

impl CliFormatter {
    /// Format system information
    pub fn format_system_info(info: &SystemInfo) -> String {
        format!(
            "System: {}, {}, {}",
            info.cpu, info.os, info.audio_backend
        )
    }

    /// Format test progress indicator
    ///
    /// Example: `[✓] 5ms: OK (0.02% underruns)`
    pub fn format_test_progress(result: &TestResult) -> String {
        let symbol = match result.verdict {
            crate::tuning::Verdict::Stable => "✓",
            crate::tuning::Verdict::Warning => "⚠",
            crate::tuning::Verdict::Unstable => "✗",
        };

        let verdict_str = match result.verdict {
            crate::tuning::Verdict::Stable => "OK",
            crate::tuning::Verdict::Warning => "WARN",
            crate::tuning::Verdict::Unstable => "FAIL",
        };

        format!(
            "[{}] {}ms interval, {} buffer: {} ({:.2}% underruns)",
            symbol,
            result.mixer_check_interval_ms,
            result.audio_buffer_size,
            verdict_str,
            result.underrun_rate()
        )
    }

    /// Format recommendations display
    ///
    /// **Traceability:** TUNE-UI-020 (interactive mode output)
    pub fn format_recommendations(recs: &Recommendations) -> String {
        let mut output = String::new();

        output.push_str("\nResults:\n");
        output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // Primary recommendation
        output.push_str(&format!("Recommended: mixer_check_interval_ms = {}\n", recs.primary.mixer_check_interval_ms));
        output.push_str(&format!("             audio_buffer_size = {}\n\n", recs.primary.audio_buffer_size));
        output.push_str(&format!("Rationale: {}\n", recs.primary.rationale));
        output.push_str(&format!("Expected latency: {:.1}ms\n", recs.primary.expected_latency_ms));
        output.push_str(&format!("Confidence: {:?}\n", recs.primary.confidence));

        output.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        output.push_str("\nAlternative (Conservative):\n");
        output.push_str(&format!("mixer_check_interval_ms = {}\n", recs.conservative.mixer_check_interval_ms));
        output.push_str(&format!("audio_buffer_size = {}\n\n", recs.conservative.audio_buffer_size));
        output.push_str(&format!("Rationale: {}\n", recs.conservative.rationale));
        output.push_str(&format!("Expected latency: {:.1}ms\n", recs.conservative.expected_latency_ms));
        output.push_str(&format!("Confidence: {:?}\n", recs.conservative.confidence));

        output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        output
    }

    /// Format curve summary table
    pub fn format_curve_summary(curve: &[CurvePoint]) -> String {
        let mut output = String::new();

        output.push_str("\nCurve Summary:\n");
        output.push_str("┌──────────┬────────────┬──────────┐\n");
        output.push_str("│ Interval │ Min Buffer │ Status   │\n");
        output.push_str("├──────────┼────────────┼──────────┤\n");

        for point in curve {
            let buffer_str = point
                .min_stable_buffer
                .map(|b| format!("{:4}", b))
                .unwrap_or_else(|| "   -".to_string());

            let status_str = match point.status {
                crate::tuning::CurveStatus::Stable => "Stable  ",
                crate::tuning::CurveStatus::Marginal => "Marginal",
                crate::tuning::CurveStatus::Unstable => "Unstable",
            };

            output.push_str(&format!(
                "│ {:4} ms   │ {} frames │ {} │\n",
                point.interval_ms, buffer_str, status_str
            ));
        }

        output.push_str("└──────────┴────────────┴──────────┘\n");

        output
    }

    /// Format phase header
    ///
    /// Example: `Phase 1: Testing mixer intervals with default buffer...`
    pub fn format_phase_header(phase: u8, description: &str) -> String {
        format!("\nPhase {}: {}...\n", phase, description)
    }

    /// Format session summary
    pub fn format_session_summary(report: &TuningReport) -> String {
        let mut output = String::new();

        output.push_str("\n╔════════════════════════════════════════╗\n");
        output.push_str("║     Buffer Auto-Tuning Complete      ║\n");
        output.push_str("╚════════════════════════════════════════╝\n\n");

        output.push_str(&format!("Duration: {} seconds\n", report.session.duration_seconds));
        output.push_str(&format!("Tests run: {}\n", report.test_results.len()));

        let stable_count = report
            .test_results
            .iter()
            .filter(|r| r.verdict == crate::tuning::Verdict::Stable)
            .count();

        output.push_str(&format!("Stable configurations: {}\n", stable_count));
        output.push_str(&format!("System: {}\n", report.system_info.cpu));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tuning::{CurveStatus, ConfidenceLevel, Recommendation, Verdict};

    fn create_test_report() -> TuningReport {
        let system_info = SystemInfo {
            cpu: "Test CPU".to_string(),
            os: "Test OS".to_string(),
            audio_backend: "Test Backend".to_string(),
            audio_device: "default".to_string(),
        };

        let test_results = vec![];
        let curve_data = vec![];
        let recommendations = None;

        TuningReport::new(system_info, test_results, curve_data, recommendations, 300)
    }

    #[test]
    fn test_create_report() {
        let report = create_test_report();

        assert_eq!(report.session.version, "1.0");
        assert_eq!(report.session.duration_seconds, 300);
        assert!(!report.session.timestamp.is_empty());
    }

    #[test]
    fn test_json_export_import() {
        let report = create_test_report();
        let temp_path = std::env::temp_dir().join("test_tuning_report.json");

        // Export
        report.export_json(&temp_path).unwrap();

        // Import
        let imported = TuningReport::import_json(&temp_path).unwrap();

        assert_eq!(imported.session.version, report.session.version);
        assert_eq!(imported.system_info.cpu, report.system_info.cpu);

        // Cleanup
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_format_system_info() {
        let info = SystemInfo {
            cpu: "AMD Ryzen 5 5600X".to_string(),
            os: "Linux 6.8.0".to_string(),
            audio_backend: "ALSA".to_string(),
            audio_device: "default".to_string(),
        };

        let formatted = CliFormatter::format_system_info(&info);
        assert!(formatted.contains("AMD Ryzen 5 5600X"));
        assert!(formatted.contains("Linux 6.8.0"));
        assert!(formatted.contains("ALSA"));
    }

    #[test]
    fn test_format_recommendations() {
        let primary = Recommendation {
            mixer_check_interval_ms: 10,
            audio_buffer_size: 256,
            expected_latency_ms: 5.8,
            confidence: ConfidenceLevel::High,
            rationale: "Test rationale".to_string(),
        };

        let conservative = Recommendation {
            mixer_check_interval_ms: 20,
            audio_buffer_size: 512,
            expected_latency_ms: 11.6,
            confidence: ConfidenceLevel::VeryHigh,
            rationale: "Conservative rationale".to_string(),
        };

        let recs = Recommendations {
            primary,
            conservative,
        };

        let formatted = CliFormatter::format_recommendations(&recs);
        assert!(formatted.contains("Recommended"));
        assert!(formatted.contains("10"));
        assert!(formatted.contains("256"));
        assert!(formatted.contains("Conservative"));
    }

    #[test]
    fn test_format_curve_summary() {
        let curve = vec![
            CurvePoint {
                interval_ms: 5,
                min_stable_buffer: Some(256),
                status: CurveStatus::Stable,
            },
            CurvePoint {
                interval_ms: 10,
                min_stable_buffer: Some(128),
                status: CurveStatus::Stable,
            },
            CurvePoint {
                interval_ms: 1,
                min_stable_buffer: None,
                status: CurveStatus::Unstable,
            },
        ];

        let formatted = CliFormatter::format_curve_summary(&curve);
        assert!(formatted.contains("Interval"));
        assert!(formatted.contains("Min Buffer"));
        assert!(formatted.contains("256"));
        assert!(formatted.contains("Stable"));
        assert!(formatted.contains("Unstable"));
    }
}
