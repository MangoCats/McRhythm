# WKMP Coding Conventions

**📝 TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines code organization and quality standards. Supports Tier 2 design documents. See [Document Hierarchy](GOV001-document_hierarchy.md).

> **Related Documentation:** [Requirements](REQ001-requirements.md) | [Architecture](SPEC001-architecture.md) | [Requirements Enumeration](GOV002-requirements_enumeration.md)

---

## Overview

This document establishes coding standards and organizational requirements for the WKMP codebase.
These conventions ensure maintainability, testability, and consistency across the Rust microservices application.

## Workspace Structure

**CO-005: Cargo Workspace Organization**

WKMP uses a Cargo workspace with multiple binary crates and a shared common library. See [Project Structure](IMPL003-project_structure.md) for complete details.

- **CO-006:** The workspace shall contain:
  - `common/` - Shared library crate (`wkmp-common`)
  - `wkmp-ap/` - Audio Player binary
  - `wkmp-ui/` - User Interface binary
  - `wkmp-le/` - Lyric Editor binary
  - `wkmp-pd/` - Program Director binary
  - `wkmp-ai/` - Audio Ingest binary (Full version only)

- **CO-007:** Shared code shall be implemented in the `common/` library:
  - Database models and queries
  - Event types (`WkmpEvent` enum)
  - API request/response types
  - Flavor calculation algorithms
  - Cooldown calculation logic
  - UUID and timestamp utilities
  - Module configuration loading

- **CO-008:** Module-specific code shall remain in respective binary crates:
  - HTTP server setup (module-specific)
  - Audio pipeline code (Audio Player only)
  - Password hashing (User Interface only)
  - Selection algorithm (Program Director only)
  - File scanning (Audio Ingest only)

- **CO-009:** Binary names shall follow the `wkmp-` prefix convention:
  - `wkmp-ap` - Audio Player
  - `wkmp-ui` - User Interface
  - `wkmp-le` - Lyric Editor
  - `wkmp-pd` - Program Director
  - `wkmp-ai` - Audio Ingest

## Code Organization and Module Structure

### Module Size and Complexity

**CO-010: Module Size Limits**

- **CO-011:** Individual source code files should not exceed 800 lines of code, excluding comments and blank lines
- **CO-012:** If a module approaches 700 lines, it should be evaluated for potential splitting into sub-modules
- **CO-013:** Modules exceeding 1000 lines require architectural review and justification documentation

**CO-020: Function Complexity**

- **CO-021:** Individual functions should not exceed 100 lines of code
- **CO-022:** Functions with [cyclomatic complexity](https://en.wikipedia.org/wiki/Cyclomatic_complexity) greater than 10 should be refactored into smaller functions

### Separation of Concerns

**CO-030: Component Module Structure**

Each module binary shall be organized into separate submodules by functional area:

- **CO-034:** Each module shall have a single, well-defined responsibility

- **CO-035:** A coordinator module (`mod.rs`) shall compose sub-modules without containing substantial logic

**CO-040: Common Library Organization**

The `common/` library crate shall be organized as follows:

- **CO-041:** Database code in `common/src/db/`:
  - `schema.rs` - Database schema types
  - `models.rs` - Data models (File, Passage, Song, etc.)
  - `queries.rs` - Common database queries
  - `migrations.rs` - Migration management

- **CO-042:** Event system in `common/src/events/`:
  - `types.rs` - `WkmpEvent` enum and related types

- **CO-043:** API types in `common/src/api/`:
  - `types.rs` - API request/response types
  - `client.rs` - HTTP client helpers

- **CO-044:** Flavor system in `common/src/flavor/`:
  - `types.rs` - FlavorVector, FlavorTarget types
  - `distance.rs` - Distance calculations (squared Euclidean)
  - `centroid.rs` - Weighted centroid calculation

- **CO-045:** Cooldown system in `common/src/cooldown/`:
  - `calculator.rs` - Cooldown multiplier calculation

- **CO-046:** Module configuration in `common/src/config/`:
  - `module.rs` - Module config loading from database

- **CO-047:** External API clients in Audio Ingest module (`wkmp-ai/src/external/`):
  - `acoustid.rs` - AcoustID fingerprinting
  - `musicbrainz.rs` - MusicBrainz API client
  - `acousticbrainz.rs` - AcousticBrainz API client
  - `essentia.rs` - Essentia FFI bindings

- **CO-048:** Audio metadata parsing in Audio Ingest module (`wkmp-ai/src/scanner/`):
  - Metadata extraction in `metadata.rs` using id3, metaflac, mp4ameta crates

- **CO-049:** Public API functions shall be clearly separated from internal implementation

### Module Dependencies

**CO-050: Dependency Direction**

- **CO-051:** Lower-level utility modules shall not depend on higher-level application modules
- **CO-052:** Module dependency graphs shall be acyclic (no circular dependencies)
- **CO-053:** Each module shall explicitly declare its dependencies via `use` statements at the top of the file
- **CO-054:** Common code shared across binary crates shall be implemented in the `common/` library
- **CO-055:** Binary crates (`wkmp-*`) shall not depend on each other; they communicate via HTTP APIs only
- **CO-056:** All binary crates shall depend on `wkmp-common` for shared types and logic

**CO-060: Interface Boundaries**

- **CO-061:** Each module shall expose a minimal public interface
- **CO-062:** Internal helper functions shall be marked as `pub(crate)` or private
- **CO-063:** Public functions shall have clear documentation comments (`///`) describing their purpose, parameters, and return values
- **CO-064:** Complex data structures crossing module boundaries shall use well-defined types (structs, enums) rather than primitive types
- **CO-065:** Use newtype patterns for domain-specific IDs (e.g., `PassageId(Uuid)`, `SongId(Uuid)`)

### Code Reusability

**CO-070: DRY (Don't Repeat Yourself)**

- **CO-071:** Duplicated code blocks (>5 lines) appearing more than twice shall be extracted into shared functions
- **CO-072:** Similar patterns with minor variations shall use parameterized functions or generic implementations
- **CO-073:** Magic numbers and strings shall be defined as named constants at module or application level
  - Example: `const DEFAULT_SONG_COOLDOWN_DAYS: i64 = 7;`
  - Example: `const MIN_QUEUE_DURATION_SECONDS: f64 = 900.0; // 15 minutes`

**CO-080: Frontend JavaScript Organization**

- **CO-081:** Frontend JavaScript code shall be organized into logical modules with clear boundaries:
  - State management (data structures, SSE connections, API clients)
  - UI rendering (DOM manipulation, component updates)
  - User interactions (event handlers, control actions)

- **CO-082:** Each JavaScript module shall use clear function organization with comment headers
- **CO-083:** Global namespace pollution shall be minimized (only essential APIs exposed globally)
- **CO-084:** Use modern JavaScript (ES6+) features: `const`/`let`, arrow functions, async/await

### Testing and Maintainability

**CO-090: Testability**

- **CO-091:** Each module shall include a `#[cfg(test)]` section with unit tests
- **CO-092:** Functions shall be designed to minimize external dependencies for easier testing
  - Use dependency injection for database connections, API clients
  - Separate pure computation from I/O operations

- **CO-093:** Test coverage for critical logic shall exceed 80%:
  - Probability calculations
  - Cooldown calculations
  - Crossfade timing logic
  - Musical flavor distance calculations
  - Selection algorithm

- **CO-094:** Each test function shall test a single, specific behavior
- **CO-095:** Integration tests shall be placed in `tests/` directory
- **CO-096:** Module-specific tests are in their respective binary crates (e.g., Audio Ingest tests in `wkmp-ai/tests/`)

**CO-100: Code Documentation**

- **CO-101:** Each module shall begin with a module-level documentation comment (`//!`) describing its purpose
- **CO-102:** Public functions shall have documentation comments in Rust standard format (`///`)
  - Include `# Arguments`, `# Returns`, `# Errors` sections where appropriate
  - Include `# Examples` for non-obvious usage

- **CO-103:** Complex algorithms shall include inline comments explaining the approach
- **CO-104:** Requirement mappings shall reference requirement IDs
  - Example: `// Implements REQ-SEL-031A: 7-day minimum song cooldown`
  - Example: `// Satisfies XFD-BEH-C1: Longer lead-in crossfade case`

- **CO-105:** Use `#[doc(hidden)]` for implementation details not part of public API

### Module Cohesion

**CO-110: Single Responsibility**

- **CO-111:** Each module shall have a single, clearly articulable responsibility
- **CO-112:** Module names shall clearly indicate their purpose
  - Good: `crossfade_engine.rs`, `probability_calculator.rs`, `queue_manager.rs`
  - Bad: `utils.rs`, `helpers.rs`, `stuff.rs`

- **CO-113:** If a module name requires "and" or multiple nouns to describe, it should be evaluated for splitting

**CO-120: Logical Grouping**

- **CO-121:** Related functionality shall be grouped in the same module or directory
- **CO-122:** Directory structure shall reflect workspace organization (see [Project Structure](IMPL003-project_structure.md)):
  ```
  /
  ├── common/            # Shared library (wkmp-common)
  │   └── src/
  │       ├── db/        # Database models and queries
  │       ├── events/    # Event types
  │       ├── api/       # API types and HTTP helpers
  │       ├── flavor/    # Flavor calculation algorithms
  │       ├── cooldown/  # Cooldown logic
  │       └── config/    # Module configuration
  ├── wkmp-ap/           # Audio Player binary
  │   └── src/
  │       ├── playback/  # Playback engine and pipeline
  │       ├── audio/     # Audio device management
  │       └── api/       # HTTP endpoints
  ├── wkmp-pd/           # Program Director binary
  │   └── src/
  │       ├── selection/ # Selection algorithm
  │       ├── timeslots/ # Timeslot management
  │       ├── monitor/   # Queue monitoring
  │       └── api/       # HTTP endpoints
  ├── wkmp-ui/           # User Interface binary
  │   └── src/
  │       ├── auth/      # Authentication and session
  │       ├── proxy/     # Proxies to other modules
  │       ├── api/       # HTTP endpoints
  │       └── static/    # Web UI assets
  ├── wkmp-le/           # Lyric Editor binary
  │   └── src/
  │       ├── ui/        # Editor and browser components
  │       ├── api/       # HTTP endpoints
  │       └── db/        # Database access
  └── wkmp-ai/           # Audio Ingest binary (Full only)
      └── src/
          ├── scanner/   # File scanning and metadata
          ├── external/  # MusicBrainz, AcoustID, Essentia
          ├── segmentation/ # Silence and boundary detection
          └── api/       # HTTP endpoints
  ```

- **CO-123:** Feature-specific code should be co-located when appropriate

### Async/Await and Concurrency

**CO-140: Async Organization**

- **CO-141:** Async functions shall be clearly marked with `async fn`
- **CO-142:** Blocking operations shall not be called from async context without `tokio::task::spawn_blocking`
- **CO-143:** Database queries shall use async database drivers when available
- **CO-144:** Channel types shall be chosen appropriately:
  - `tokio::sync::mpsc` for async message passing
  - `std::sync::mpsc` for sync contexts only
  - `tokio::sync::broadcast` for SSE broadcasting

- **CO-145:** Mutex types shall be chosen appropriately:
  - `tokio::sync::RwLock` for async contexts
  - `std::sync::RwLock` for sync contexts or when crossing sync/async boundary

**CO-150: Error Handling in Async Code**

- **CO-151:** Async functions shall return `Result<T, E>` for recoverable errors
- **CO-152:** Use `?` operator for error propagation in async functions
- **CO-153:** Background tasks shall log errors that cannot be propagated
- **CO-154:** Use `tokio::select!` carefully with cancellation safety in mind

### Error Handling

**CO-160: Error Types and Handling**

- **CO-161:** Error handling shall be consistent within a module
- **CO-162:** Expected errors shall use `Result<T, E>` types with meaningful error enums/structs
  - Define custom error types using `thiserror` crate
  - Example: `SelectionError`, `PlaybackError`, `DatabaseError`

- **CO-163:** Use `anyhow::Result` for application-level error handling where specific error types aren't needed
- **CO-164:** Unexpected errors that should not occur shall use `panic!` with descriptive messages
- **CO-165:** Error messages shall be logged with appropriate severity levels using `tracing` crate:
  - `trace!` - Fine-grained execution flow
  - `debug!` - Detailed operation info
  - `info!` - State changes, significant events
  - `warn!` - Recoverable issues, missing data
  - `error!` - System failures, data corruption

- **CO-166:** Network errors shall implement retry logic as specified in `REQ-NET-010`
- **CO-167:** File I/O errors shall be handled gracefully (skip file, log, continue)

### Database Code

**CO-170: Database Access Patterns**

- **CO-171:** All database queries shall use parameterized queries (no string concatenation)
- **CO-172:** Database transactions shall be used for multi-step operations
- **CO-173:** Database connection pools shall be used for concurrent access
- **CO-174:** Migration code shall be separated from application code
- **CO-175:** Database schema shall be versioned and migrations shall be tested

**CO-180: Query Organization**

- **CO-181:** Complex queries shall be defined as named constants or functions
- **CO-182:** Queries used in multiple places shall be in a shared module
- **CO-183:** Use SQLite JSON1 extension for musical flavor vector queries
- **CO-184:** Database triggers shall be documented in code comments

### API and Interface Contracts

**CO-200: API Stability**

- **CO-201:** Public module interfaces shall maintain backward compatibility where possible
- **CO-202:** Breaking changes to module interfaces shall be documented in module comments
- **CO-203:** Deprecated functions shall be marked with `#[deprecated]` and provide migration guidance
- **CO-204:** REST API endpoints shall maintain backward compatibility
  - Add new fields, don't remove existing ones
  - Version API if breaking changes are necessary (`/api/v2/...`)

**CO-210: Tauri Command Handlers**

- **CO-211:** Tauri commands shall have clear error handling
- **CO-212:** Tauri commands shall validate input parameters
- **CO-213:** Tauri commands shall return strongly-typed results
- **CO-214:** Long-running operations shall use async Tauri commands
- **CO-215:** Tauri state shall use `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared state

### Logging and Tracing

**CO-220: Logging Standards**

- **CO-221:** Use `tracing` crate for all logging (not `log` crate)
- **CO-222:** Configure tracing to include file and line numbers (as per `REQ-ERR-010`)
- **CO-223:** Use structured logging with fields:
  ```rust
  tracing::info!(
      passage_id = %passage.id,
      duration = passage.duration,
      "Starting playback"
  );
  ```

- **CO-224:** Instrument async functions with `#[tracing::instrument]` where appropriate
- **CO-225:** Log levels shall be chosen appropriately:
  - Don't log at `info` level in hot loops
  - Use `debug` for detailed diagnostics
  - Use `warn` for recoverable issues
  - Use `error` for failures requiring attention

- **CO-226:** Each binary shall log build identification information at startup at INFO level containing:
  - Cargo package version from `Cargo.toml`
  - Git commit hash (short form, 8 characters)
  - Build timestamp (ISO 8601 format)
  - Build profile (debug/release)
  - Example output: `wkmp-ap v0.1.0 [a1b2c3d4] built 2025-10-19T12:34:56Z (debug)`
  - This enables unambiguous determination of which code version is running

### Testing Conventions

**CO-230: Test Organization**

- **CO-231:** Unit tests shall be in `#[cfg(test)]` module within the same file
- **CO-232:** Integration tests shall be in `tests/` directory
- **CO-233:** Test fixtures and helpers shall be in `tests/common/` module
- **CO-234:** Mock objects shall be clearly named (e.g., `MockDatabaseConnection`)
- **CO-235:** Use `#[tokio::test]` for async tests

**CO-240: Test Naming and Structure**

- **CO-241:** Test functions shall have descriptive names indicating what is tested:
  - `test_song_cooldown_zero_during_minimum_period()`
  - `test_crossfade_case1_longer_lead_in()`
  - `test_selection_excludes_zero_probability_passages()`

- **CO-242:** Tests shall follow Arrange-Act-Assert pattern
- **CO-243:** Tests shall be independent and not rely on execution order
- **CO-244:** Tests shall clean up resources (temp files, database connections)

### Performance Considerations

**CO-250: Performance Guidelines**

- **CO-251:** Avoid unnecessary cloning of large data structures
  - Use references (`&`) when ownership transfer is not needed
  - Use `Arc` for shared ownership across threads

- **CO-252:** Database queries in loops shall be batched when possible
- **CO-253:** Musical flavor distance calculations shall be optimized for the top-100 selection
- **CO-255:** Memory allocations in audio callback paths shall be minimized

**CO-260: Raspberry Pi Optimization**

- **CO-261:** Be mindful of memory usage on resource-constrained devices
- **CO-262:** Avoid unnecessary buffering of large audio data
- **CO-264:** Test performance on actual target hardware, not just desktop

### Requirements Traceability

**CO-270: Traceability to Requirements**

- **CO-271:** Where code implements a specific requirement, include the requirement ID in a comment:
  ```rust
  // Implements REQ-SEL-031A: 7-day minimum song cooldown default
  const DEFAULT_SONG_COOLDOWN: i64 = 7 * 24 * 60 * 60; // 7 days in seconds
  ```

- **CO-272:** Complex algorithms shall reference design documents:
  ```rust
  // Implements musical flavor distance calculation as specified in FLV-DIST-010
  fn calculate_flavor_distance(a: &FlavorVector, b: &FlavorVector) -> f64 {
      // ...
  }
  ```

- **CO-273:** Crossfade timing logic shall reference crossfade specification:
  ```rust
  // Implements XFD-BEH-C1: Case 1 - Following passage has longer lead-in
  if current_lead_out_duration <= next_lead_in_duration {
      // ...
  }
  ```

- **CO-274:** When implementing multiple related requirements, list them:
  ```rust
  // Implements:
  // - REQ-SEL-070: Cooldown stacking (multiply probabilities)
  // - REQ-SEL-031: Song cooldown ramping
  // - REQ-SEL-041: Artist cooldown ramping
  let net_probability = base_prob * song_mult * artist_mult * work_mult;
  ```

## Code Style

### Rust Formatting

- **CO-280:** Use `rustfmt` with default settings for all Rust code
- **CO-281:** Use `clippy` and address all warnings
- **CO-282:** Follow Rust naming conventions:
  - `snake_case` for functions, variables, modules
  - `CamelCase` for types, traits, enums
  - `SCREAMING_SNAKE_CASE` for constants
  - `'lowercase` for lifetimes

- **CO-283:** Maximum line length: 100 characters (rustfmt default)

### Comments and Documentation

- **CO-290:** Prefer self-documenting code over comments where possible
- **CO-291:** Comments shall explain "why", not "what" (the code shows "what")
- **CO-292:** TODO comments shall include context and owner:
  ```rust
  // TODO(username): Implement work cooldown after specification finalized (see IMPL-P5-19)
  ```

- **CO-293:** FIXME comments shall include issue tracking reference when applicable
- **CO-294:** Avoid commented-out code in production branches

- **CO-299:** "WKMP" is the project name. "WKMP" and other 
              references to the project name shall not appear anywhere in the source code.
              The public facing name of the project while coding shall be: "WKMP" ; 
              in long form: "Wonderfully Kinetic Music Player".

## Review and Approval

**Document Status:** Draft
**Version:** 1.0
**Last Updated:** 2025-10-05
**Author:** WKMP Development Team

**Change History:**
- 2025-10-05: Initial version adapted from coding conventions specification

----
End of document - WKMP Coding Conventions
