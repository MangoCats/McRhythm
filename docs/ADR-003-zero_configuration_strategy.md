# ADR-003: Zero-Configuration Startup Strategy

**Status:** Accepted
**Date:** 2025-10 (Decision), 2025-11-02 (Documented)
**Deciders:** WKMP Development Team
**Related Requirements:** [REQ-NF-030] through [REQ-NF-037]

---

## Context

All 6 WKMP microservices must start without requiring configuration files, satisfying [REQ-NF-030] through [REQ-NF-037]. The system needs to:

1. **Work out-of-the-box** for 95% of users (typical installation)
2. **Allow customization** for power users with non-standard setups
3. **Support cross-platform** installations (Windows, macOS, Linux)
4. **Avoid user friction** during first-time startup

**Key Challenge:** How to determine where to store the shared SQLite database (`wkmp.db`) without asking users to edit configuration files?

---

## Decision

Implement a **4-tier priority system** for root folder resolution:

### Resolution Priority (Highest to Lowest)

1. **CLI argument:** `--root-folder /custom/path` or `--root /custom/path`
2. **Environment variable:** `WKMP_ROOT_FOLDER=/custom/path` or `WKMP_ROOT=/custom/path`
3. **TOML config file:** `~/.config/wkmp/<module-name>.toml` (XDG standard)
4. **Compiled default:** `~/Music` (Linux/macOS), `%USERPROFILE%\Music` (Windows)

### Implementation Pattern

All modules use shared utilities from `wkmp_common::config`:

```rust
use wkmp_common::config::{RootFolderResolver, RootFolderInitializer};

// Step 1: Resolve root folder (4-tier priority)
let resolver = RootFolderResolver::new("module-name");
let root_folder = resolver.resolve();

// Step 2: Create directory if missing
let initializer = RootFolderInitializer::new(root_folder);
initializer.ensure_directory_exists()?;

// Step 3: Get database path
let db_path = initializer.database_path();  // root_folder/wkmp.db
```

### Directory Structure

```
~/Music/                          # Default root folder
├── wkmp.db                       # SQLite database
├── audio_files/                  # User's music library (not managed by WKMP)
└── .wkmp/                        # WKMP internal (future use)
```

**Note:** WKMP only creates/manages `wkmp.db`, not the music files themselves.

---

## Consequences

### Positive

✅ **Zero-config for 95% of users**
- Typical users just install and run
- No "Edit config file to get started" friction
- Default location (`~/Music`) is intuitive for music software

✅ **Power users can override**
- CLI args for quick testing (`wkmp-ap --root /tmp/test`)
- ENV vars for deployment (`WKMP_ROOT_FOLDER=/data/wkmp`)
- TOML config for persistent overrides

✅ **Cross-platform compatibility**
- Windows: `%USERPROFILE%\Music`
- macOS: `~/Music`
- Linux: `~/Music`

✅ **Testable**
- Unit tests can override via CLI args
- CI/CD can use ENV vars
- No config file pollution during tests

✅ **Discoverability**
- Users can find database with simple path (`~/Music/wkmp.db`)
- Backup/restore is straightforward
- Database Review tool (wkmp-dr) works immediately

### Negative

❌ **Multiple resolution mechanisms add complexity**
- 4 tiers must be checked in order
- Potential for confusion if users set multiple overrides
- Testing requires coverage of all 4 tiers

❌ **TOML dependency even though rarely used**
- Must include `toml` crate in dependencies
- Config file parsing logic even for users who never use it
- Maintenance burden for rarely-exercised code path

❌ **Cannot detect if user MEANT to use non-standard location**
- If `/opt/wkmp` was intended but typo in ENV var → falls back to default
- Silent fallback might surprise users
- No warning if config file has syntax errors (silently ignored)

❌ **XDG config path may not exist on fresh installs**
- `~/.config/wkmp/` must be created by user or module
- Linux users expect XDG, Windows users expect AppData
- Cross-platform path resolution has edge cases

### Mitigations

**Logging:**
```rust
info!("Root folder resolved to: {}", root_folder.display());
info!("Resolution method: {:?}", resolution_method);  // CLI, ENV, TOML, or Default
```

**Validation:**
```rust
if !root_folder.exists() {
    warn!("Root folder {} does not exist, creating...", root_folder.display());
    initializer.ensure_directory_exists()?;
}
```

**Documentation:**
- [SPEC001: Architecture](SPEC001-architecture.md#6-zero-configuration-startup) explains resolution order
- [DEV_QUICKSTART.md](DEV_QUICKSTART.md) mentions CLI args for testing
- Error messages include resolution path: "Database not found at <path> (resolved via <method>)"

---

## Alternatives Considered

### Alternative 1: Hardcoded `~/Music` Only

**Approach:** Always use `~/Music`, no overrides.

**Pros:**
- Simplest implementation (5 lines of code)
- Zero ambiguity
- No config file parsing

**Cons:**
- ❌ **No override for non-standard setups** (e.g., NAS, `/data` partition)
- ❌ **Testing requires filesystem manipulation** (no CLI override)
- ❌ **CI/CD inflexible** (must use ~/Music in containers)

**Decision:** Rejected - No override mechanism fails 5% power user requirement

---

### Alternative 2: Environment Variable Only

**Approach:** Use `WKMP_ROOT_FOLDER` env var, fall back to `~/Music`.

**Pros:**
- Simple (no CLI parsing, no TOML)
- Good for CI/CD and deployment
- Discoverable via `env | grep WKMP`

**Cons:**
- ❌ **No per-run override** (CLI args more convenient for testing)
- ❌ **Poor discoverability** (users don't know to check ENV vars)
- ❌ **Persistence requires shell config** (must edit .bashrc/.zshrc)

**Decision:** Rejected - Worse user experience than 4-tier system

---

### Alternative 3: Config File Required

**Approach:** Require `~/.config/wkmp/config.toml` with `root_folder = "/path"`.

**Pros:**
- Explicit configuration (no ambiguity)
- Standard for many Linux tools
- Centralized settings location

**Cons:**
- ❌ **Violates zero-config goal** ([REQ-NF-030])
- ❌ **User friction** ("Edit this file to get started")
- ❌ **First-time startup fails** (no default value)

**Decision:** Rejected - Contradicts core requirement (zero-config)

---

### Alternative 4: Ask User on First Run

**Approach:** If no config detected, prompt user to choose location.

**Pros:**
- User explicitly chooses location
- No "magic" defaults
- Opportunity to educate user

**Cons:**
- ❌ **Violates zero-config goal** (requires interaction)
- ❌ **Blocks automated deployments** (CI/CD requires human input)
- ❌ **Poor CLI tool experience** (headless servers can't prompt)

**Decision:** Rejected - Incompatible with automated deployment and headless operation

---

## Implementation Details

### RootFolderResolver (wkmp_common::config)

```rust
pub struct RootFolderResolver {
    module_name: String,
}

impl RootFolderResolver {
    pub fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
        }
    }

    pub fn resolve(&self) -> PathBuf {
        // Tier 1: CLI args (via clap)
        if let Some(path) = self.check_cli_args() {
            return path;
        }

        // Tier 2: Environment variables
        if let Ok(path) = env::var("WKMP_ROOT_FOLDER") {
            return PathBuf::from(path);
        }
        if let Ok(path) = env::var("WKMP_ROOT") {
            return PathBuf::from(path);
        }

        // Tier 3: TOML config file
        if let Some(path) = self.check_toml_config() {
            return path;
        }

        // Tier 4: Compiled default
        self.default_root_folder()
    }

    fn default_root_folder(&self) -> PathBuf {
        if cfg!(target_os = "windows") {
            PathBuf::from(env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string()))
                .join("Music")
        } else {
            PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
                .join("Music")
        }
    }
}
```

### RootFolderInitializer (wkmp_common::config)

```rust
pub struct RootFolderInitializer {
    root_folder: PathBuf,
}

impl RootFolderInitializer {
    pub fn new(root_folder: PathBuf) -> Self {
        Self { root_folder }
    }

    pub fn ensure_directory_exists(&self) -> Result<()> {
        if !self.root_folder.exists() {
            fs::create_dir_all(&self.root_folder)?;
            info!("Created root folder: {}", self.root_folder.display());
        }
        Ok(())
    }

    pub fn database_path(&self) -> PathBuf {
        self.root_folder.join("wkmp.db")
    }
}
```

---

## Compliance Matrix

| Requirement | Tier | Compliance |
|-------------|------|------------|
| [REQ-NF-030] wkmp-ap zero-config | Tier 4 (default) | ✅ |
| [REQ-NF-031] wkmp-ui zero-config | Tier 4 (default) | ✅ |
| [REQ-NF-032] wkmp-pd zero-config | Tier 4 (default) | ✅ |
| [REQ-NF-033] wkmp-ai zero-config | Tier 4 (default) | ✅ |
| [REQ-NF-034] wkmp-le zero-config | Tier 4 (default) | ✅ |
| [REQ-NF-035] wkmp-dr zero-config | Tier 4 (default) | ✅ |
| Power user override | Tiers 1-3 | ✅ |
| Cross-platform | All tiers | ✅ |
| Testability | Tier 1 (CLI) | ✅ |

---

## Testing Strategy

### Unit Tests (wkmp_common::config)

```rust
#[test]
fn test_cli_arg_overrides_all() {
    let resolver = RootFolderResolver::new("test-module");
    // Simulate CLI arg: --root-folder /custom
    let path = resolver.resolve_with_cli_arg(Some("/custom".to_string()));
    assert_eq!(path, PathBuf::from("/custom"));
}

#[test]
fn test_env_var_overrides_toml() {
    env::set_var("WKMP_ROOT_FOLDER", "/env");
    let resolver = RootFolderResolver::new("test-module");
    let path = resolver.resolve();
    assert_eq!(path, PathBuf::from("/env"));
}

#[test]
fn test_default_fallback() {
    env::remove_var("WKMP_ROOT_FOLDER");
    env::remove_var("WKMP_ROOT");
    let resolver = RootFolderResolver::new("test-module");
    let path = resolver.resolve();
    assert!(path.ends_with("Music"));
}
```

### Integration Tests (Per Module)

```rust
#[tokio::test]
async fn tc_i_nf010_01_zero_config_via_cli() {
    // Test: Module starts with --root-folder /tmp/test
    let output = Command::new("wkmp-dr")
        .arg("--root-folder")
        .arg("/tmp/test")
        .spawn()
        .expect("Failed to start");

    // Verify: Database created at /tmp/test/wkmp.db
    assert!(PathBuf::from("/tmp/test/wkmp.db").exists());
}
```

---

## Future Considerations

### XDG Base Directory Specification (Linux)

**Current:** TOML config at `~/.config/wkmp/<module>.toml`

**Future:** Full XDG support:
- `XDG_CONFIG_HOME/wkmp/<module>.toml` (config)
- `XDG_DATA_HOME/wkmp/` (database, if not in ~/Music)
- `XDG_CACHE_HOME/wkmp/` (logs, temporary files)

**Reason for deferral:** Adds complexity, 95% of users don't need it

### Windows Registry Support

**Current:** No Windows Registry integration

**Future:** Check `HKCU\Software\WKMP\RootFolder` as Tier 2.5 (between ENV and TOML)

**Reason for deferral:** ENV vars sufficient for Windows power users

### Database Auto-Migration

**Current:** Database must be in `<root_folder>/wkmp.db`

**Future:** Detect old location, offer to migrate

**Example:**
```
Old: ~/.wkmp/wkmp.db (pre-v1.0)
New: ~/Music/wkmp.db (v1.0+)
Prompt: "Database found at old location, migrate to ~/Music? (y/n)"
```

---

## References

- [SPEC001: Architecture](SPEC001-architecture.md#6-zero-configuration-startup)
- [REQ-NF-030] through [REQ-NF-037] in [REQ001](REQ001-requirements.md)
- [IMPL003: Project Structure](IMPL003-project_structure.md)
- [DWI001: Workflow Quickstart](../workflows/DWI001_workflow_quickstart.md)

---

**Status:** Accepted and Implemented (All 6 modules compliant as of 2025-11)
**Review Date:** Next major architecture change or v2.0 planning
