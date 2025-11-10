//! Database access for wkmp-ai
//!
//! **[AIA-DB-010]** Shared SQLite database access
//!
//! Database initialization is now handled by wkmp_common::db::init::init_database()
//! per REQ-NF-037 to ensure all modules create the complete shared schema.

pub mod albums;
pub mod artists;
pub mod files;
pub mod parameters;
pub mod passages;
pub mod schema;  // PLAN024 TASK-003: Schema synchronization
pub mod sessions;
pub mod settings;
pub mod songs;
pub mod works;
