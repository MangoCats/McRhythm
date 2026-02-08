# PLAN031 Dependencies Map

## Existing Code Dependencies

| Dependency | Status | Purpose | Risk |
|------------|--------|---------|------|
| `wkmp-ai/src/services/workflow_orchestrator.rs` | Exists | Session hook integration | Low |
| `wkmp-ai/src/fusion/identity_resolver.rs` | Exists | Fusion hook integration | Low |
| `wkmp-ai/src/services/content_type_classifier.rs` | Exists | Classification hook integration | Low |
| `wkmp-ai/src/db/` | Exists | Database operations pattern | Low |
| `common/src/db/init.rs` | Exists | Migration framework | Low |
| `migrations/` | Exists | Schema migrations | Low |

## New Code to Create

| Component | Location | Dependencies |
|-----------|----------|--------------|
| `ground_truth.rs` | `wkmp-ai/src/testing/` | Database, serde |
| `evaluation.rs` | `wkmp-ai/src/testing/` | ground_truth |
| `batch_orchestrator.rs` | `wkmp-ai/src/testing/` | evaluation, existing pipeline |
| `failure_analyzer.rs` | `wkmp-ai/src/testing/` | evaluation |
| `reporting.rs` | `wkmp-ai/src/testing/` | evaluation, batch_orchestrator |
| `cli.rs` | `wkmp-ai/src/testing/` | All above |
| `mod.rs` | `wkmp-ai/src/testing/` | Module exports |

## External Dependencies

| Dependency | Version | Purpose | Already in Cargo.toml? |
|------------|---------|---------|------------------------|
| `serde` | 1.x | JSON serialization | Yes |
| `serde_json` | 1.x | JSON parsing | Yes |
| `sqlx` | 0.7.x | Database operations | Yes |
| `tokio` | 1.x | Async runtime | Yes |
| `tracing` | 0.1.x | Logging | Yes |
| `chrono` | 0.4.x | Timestamps | Yes |
| `clap` | 4.x | CLI parsing | Yes |
| `sha2` | 0.10.x | File hashing | Yes |

## Database Dependencies

| Table | Status | Relationship |
|-------|--------|--------------|
| `ground_truth` | NEW | Standalone (no FK to other tables) |
| `test_runs` | NEW | References ground_truth |
| `test_results` | NEW | References test_runs, ground_truth |
| `files` | Exists | Queried for file_hash |
| `passages` | Exists | Queried for assigned MBIDs |
