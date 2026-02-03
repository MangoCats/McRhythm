//! Closed-loop algorithm improvement testing system
//!
//! **[PLAN031]** Test-driven feedback system for improving MBID assignment accuracy
//!
//! This module provides infrastructure for:
//! - Loading and managing ground truth data
//! - Running test batches against the import pipeline
//! - Evaluating accuracy with TP/FP/TN/FN classification
//! - Analyzing failure patterns
//! - Generating improvement suggestions
//! - Tracking progress across batches
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    CLOSED-LOOP CYCLE                            │
//! │                                                                 │
//! │  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
//! │  │  IMPORT  │───▶│ EVALUATE │───▶│ ANALYZE  │───▶│ IMPROVE  │  │
//! │  │  BATCH   │    │ RESULTS  │    │ FAILURES │    │ ALGORITHM│  │
//! │  └──────────┘    └──────────┘    └──────────┘    └──────────┘  │
//! │       ▲                                               │         │
//! │       └───────────────────────────────────────────────┘         │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use wkmp_ai::testing::{BatchConfig, TestPhase};
//!
//! // Create Phase 1 configuration
//! let config = BatchConfig::phase1();
//!
//! // Run test batch (future increment)
//! // let report = orchestrator.process_batch(&config).await?;
//! ```
//!
//! ## Modules
//!
//! - `types` - Core types (Classification, FailureCategory, BatchConfig, etc.)
//! - `ground_truth` - Ground truth data management (Increment 2)
//! - `evaluation` - Result evaluation engine (Increment 3)
//! - `batch` - Batch orchestration (Increment 4)
//! - `failure` - Failure pattern analysis (Increment 6)
//! - `reset` - Database reset operations (Increment 7)
//! - `reporting` - Report generation (Increment 8)

pub mod batch;
pub mod evaluation;
pub mod failure;
pub mod ground_truth;
pub mod hooks;
pub mod import_harness;
#[cfg(test)]
mod integration_tests;
pub mod reporting;
pub mod reset;
pub mod types;

// Re-export core types for convenience
pub use types::{
    AlbumClassification, BatchConfig, Classification, FailureCategory, ResetMode, StopReason,
    TestPhase,
};

// Re-export ground truth types
pub use ground_truth::{GroundTruth, VerificationMethod};

// Re-export evaluation types
pub use evaluation::{EvaluationEngine, EvaluationMetrics, ImportResult};

// Re-export batch orchestration types
pub use batch::{BatchReport, RunId, RunStatus, TestFileProcessor, TestOrchestrator};

// Re-export pipeline hooks types
pub use hooks::{BatchSummary, FileSummary, ImportEvent, TestEvaluator};

// Re-export failure analysis types
pub use failure::{FailureAnalysis, FailureAnalyzer, FailurePattern, ImprovementSuggestion};

// Re-export reset functions
pub use reset::{reset_for_testing, ResetResult, TestStatistics};

// Re-export reporting types
pub use reporting::{ProgressReport, ReportFormatter};

// Re-export import harness types
pub use import_harness::{
    EvaluatedImportResult, ImportHarnessConfig, ImportTestHarness, ImportTestReport,
    ImportTestResult, ImportTestStats,
};
