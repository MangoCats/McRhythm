//! Database Schema Validation Tests
//!
//! Tests that verify SPEC031 zero-conf database schema

mod helpers;

// Re-export database schema tests
mod integration {
    pub mod database_schema_tests;
}
