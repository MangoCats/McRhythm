# McRhythm Coding Conventions

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [Requirements Enumeration](requirements_enumeration.md)

## Overview

This document establishes coding standards and organizational requirements for the McRhythm codebase.
These conventions ensure maintainability, testability, and consistency across the Rust/GStreamer/Tauri application.

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

Each major component shall be organized into separate modules by functional area:

- **CO-031:** Playback controller modules:
  - `playback/controller.rs` - Main playback coordination
  - `playback/gstreamer_pipeline.rs` - GStreamer pipeline management
  - `playback/crossfade.rs` - Crossfade timing and volume control
  - `playback/fade_curves.rs` - Fade curve implementations (exponential, cosine, linear)

- **CO-032:** Program director modules:
  - `selection/director.rs` - Main selection coordination
  - `selection/probability.rs` - Probability calculation logic
  - `selection/cooldown.rs` - Cooldown calculation logic
  - `selection/flavor_distance.rs` - Musical flavor distance calculations

- **CO-033:** Web UI modules:
  - `ui/tauri_handlers.rs` - Tauri command handlers
  - `ui/sse.rs` - Server-sent events broadcasting
  - `ui/api.rs` - REST API endpoint definitions
  - Frontend assets in `src-tauri/frontend/` directory

- **CO-034:** Each module shall have a single, well-defined responsibility

- **CO-035:** A coordinator module (`mod.rs`) shall compose sub-modules without containing substantial logic

**CO-040: Backend Module Organization**

- **CO-041:** Database access logic shall be separated from business logic
  - Database queries in `db/queries.rs`, `db/schema.rs`
  - Business logic in component-specific modules

- **CO-042:** External API clients shall be isolated in dedicated modules
  - `external/acoustid.rs` - AcoustID fingerprinting
  - `external/musicbrainz.rs` - MusicBrainz API client
  - `external/acousticbrainz.rs` - AcousticBrainz API client
  - `external/listenbrainz.rs` - ListenBrainz API client (future)

- **CO-043:** Audio metadata parsing shall be isolated in dedicated modules
  - `metadata/id3.rs` - ID3v2 tag parsing
  - `metadata/vorbis.rs` - Vorbis comment parsing
  - `metadata/mp4.rs` - MP4/M4A tag parsing

- **CO-044:** Public API functions shall be clearly separated from internal implementation

### Module Dependencies

**CO-050: Dependency Direction**

- **CO-051:** Lower-level utility modules shall not depend on higher-level application modules
- **CO-052:** Module dependency graphs shall be acyclic (no circular dependencies)
- **CO-053:** Each module shall explicitly declare its dependencies via `use` statements at the top of the file
- **CO-054:** Common code shared across modules shall be extracted to a `common` or `utils` module

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
- **CO-096:** Use `#[cfg(feature = "full")]` / `#[cfg(feature = "lite")]` for version-specific tests

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
- **CO-122:** Directory structure shall reflect functional organization:
  ```
  src/
  ├── playback/          # Playback controller and audio engine
  ├── selection/         # Program director and selection logic
  ├── queue/             # Queue manager
  ├── history/           # Historian and play tracking
  ├── flavor/            # Flavor manager and timeslots
  ├── library/           # Library scanner and file management
  ├── metadata/          # Metadata extraction
  ├── external/          # External API clients
  ├── db/                # Database schema and queries
  ├── ui/                # Tauri handlers and API
  └── common/            # Shared utilities
  ```

- **CO-123:** Feature-specific code should be co-located when appropriate

### GStreamer Integration

**CO-130: GStreamer Code Organization**

- **CO-131:** GStreamer pipeline creation shall be separated from pipeline control
- **CO-132:** GStreamer bus message handling shall be in a dedicated event loop
- **CO-133:** Volume control and fade automation shall be encapsulated in separate functions
- **CO-134:** Pipeline state transitions shall use proper error handling (all state change results checked)
- **CO-135:** GStreamer element creation shall check for `None` and return appropriate errors

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

### Version-Specific Code

**CO-190: Feature Flag Usage**

- **CO-191:** Version-specific code shall use Rust feature flags:
  - `#[cfg(feature = "full")]` - Full version only
  - `#[cfg(feature = "lite")]` - Lite and Full versions
  - `#[cfg(feature = "minimal")]` - All versions

- **CO-192:** Feature-gated modules shall have clear documentation about version applicability
- **CO-193:** Shared code shall be implemented in a way that works across all versions
- **CO-194:** Version-specific functionality shall degrade gracefully
  - Example: Lite version shows read-only UI instead of edit controls

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
- **CO-254:** GStreamer pipeline creation shall happen off the critical path
- **CO-255:** Memory allocations in audio callback paths shall be minimized

**CO-260: Raspberry Pi Optimization**

- **CO-261:** Be mindful of memory usage on resource-constrained devices
- **CO-262:** Avoid unnecessary buffering of large audio data
- **CO-263:** Use appropriate GStreamer buffer sizes for Pi Zero2W
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

## Review and Approval

**Document Status:** Draft
**Version:** 1.0
**Last Updated:** 2025-10-05
**Author:** McRhythm Development Team

**Change History:**
- 2025-10-05: Initial version adapted from coding conventions specification
