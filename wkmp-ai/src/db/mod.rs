//! Database access for wkmp-ai
//!
//! **[AIA-DB-010]** Shared SQLite database access
//!
//! Database initialization is now handled by wkmp_common::db::init::init_database()
//! per REQ-NF-037 to ensure all modules create the complete shared schema.

pub mod acoustid_cache; // SSI-CACHE-020: AcoustID response cache
pub mod albums;
pub mod artists;
pub mod duration_cache; // SSI-DUR-010: Accurate duration cache for VBR MP3s
pub mod files;
pub mod fingerprint_cache; // SSI-CACHE-010: Chromaprint fingerprint cache
pub mod parameters;
pub mod passages;
pub mod recording_cache; // PLAN026 Increment 3: MusicBrainz recording cache
pub mod schema; // PLAN024 TASK-003: Schema synchronization
pub mod sessions;
pub mod settings;
pub mod songs;
pub mod works;
