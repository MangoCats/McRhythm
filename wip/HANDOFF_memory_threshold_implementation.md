# Handoff Plan: Implement ai_memory_usage_threshold_bytes Parameter

**Task:** Replace hardcoded memory threshold with configurable database parameter
**Priority:** HIGH - Currently causing false warnings in production
**Estimated Time:** 2-3 hours
**Created:** 2025-11-16

---

## Context

The memory monitor in `wkmp-ai` currently uses a hardcoded 500MB warning threshold (with 1GB critical threshold at 2x). This causes false warnings on systems with ample RAM. We've already added the parameter to IMPL016-settings_reference.md documentation, now need to implement the code changes.

**Problem Example from testV.log:**
```
2025-11-16T23:25:38.383606Z ERROR ... CRITICAL: Memory usage 3671MB exceeds 2x threshold (1000MB)
```

The user's system has 32GB RAM, so 3.6GB usage is not concerning. The new parameter `ai_memory_usage_threshold_bytes` with 12GB default will fix this.

---

## Implementation Steps

### Step 1: Add Parameter to Database Initialization (15 minutes)

**File:** `wkmp-common/src/db/init.rs`

**Location:** After line 259 (where `ai_longwork_yield_interval_ms` is set)

**Add:**
```rust
// **[IMPL016]** Memory usage threshold for monitoring
// Process memory threshold in bytes. When exceeded, warnings are logged.
// Default: 12GB (12884901888 bytes) - appropriate for modern systems with 16+ GB RAM
// RESTART_REQUIRED - Read during bootstrap initialization
ensure_setting(pool, "ai_memory_usage_threshold_bytes", "12884901888").await?;
```

---

### Step 2: Add to Bootstrap Configuration Structure (30 minutes)

**File:** `wkmp-ai/src/models/bootstrap_config.rs`

**Changes:**

1. **Add field to struct** (after line 55):
```rust
/// Memory usage threshold in bytes for monitoring
///
/// **Default:** 12GB (12884901888 bytes)
/// **Purpose:** Threshold for memory usage warnings/critical alerts
/// **Rationale:** Modern systems have 16-32GB RAM, 12GB is reasonable operating limit
pub memory_usage_threshold_bytes: u64,
```

2. **Update SQL query** (modify lines 92-120):
Add to the SELECT statement:
```sql
COALESCE(
    (SELECT value FROM settings WHERE key = 'ai_memory_usage_threshold_bytes'),
    '12884901888'
) as memory_threshold
```

3. **Parse the value** (around line 130, after processing_thread_count):
```rust
// Parse memory threshold with validation
let memory_usage_threshold_bytes: u64 = row
    .try_get::<String, _>("memory_threshold")
    .context("Failed to read memory_threshold")?
    .parse()
    .context("Invalid ai_memory_usage_threshold_bytes value")?;

// Validate reasonable range (1GB - 128GB)
if memory_usage_threshold_bytes < 1_073_741_824 {
    tracing::warn!("Memory threshold {}MB is very low, using 1GB minimum",
        memory_usage_threshold_bytes / 1_000_000);
    let memory_usage_threshold_bytes = 1_073_741_824;
} else if memory_usage_threshold_bytes > 137_438_953_472 {
    tracing::warn!("Memory threshold {}GB exceeds 128GB maximum, capping",
        memory_usage_threshold_bytes / 1_000_000_000);
    let memory_usage_threshold_bytes = 137_438_953_472;
}
```

4. **Return in struct** (around line 145):
```rust
Ok(Self {
    connection_pool_size,
    lock_retry_ms,
    max_lock_wait_ms,
    processing_thread_count,
    memory_usage_threshold_bytes,  // Add this line
})
```

---

### Step 3: Update Memory Monitor Constructor (20 minutes)

**File:** `wkmp-ai/src/utils/memory_monitor.rs`

**Changes:**

1. **Modify `with_threshold` to accept bytes** (line 60):
```rust
/// Create monitor with custom warning threshold
///
/// # Arguments
/// * `warning_threshold_bytes` - Warning threshold in bytes
pub fn with_threshold(warning_threshold_bytes: u64) -> Self {
    let pid = sysinfo::get_current_pid().expect("Failed to get current PID");

    Self {
        system: Arc::new(RwLock::new(System::new())),
        pid,
        high_water_mark: Arc::new(AtomicU64::new(0)),
        warning_threshold: warning_threshold_bytes,
    }
}
```

2. **Note:** The critical threshold logic (2x warning) at line 94 remains unchanged. This means:
   - Warning threshold: 12GB
   - Critical threshold: 24GB (2x warning)

---

### Step 4: Pass Parameter to Memory Monitor (45 minutes)

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

**Current code (line 164):**
```rust
memory_monitor: Arc::new(crate::utils::MemoryMonitor::new()),
```

**Problem:** The WorkflowOrchestrator doesn't currently have access to the bootstrap config. Need to pass it through.

**Solution A (Recommended): Add to WorkflowOrchestrator::new()**

1. **Find WorkflowOrchestrator::new()** (search for `impl WorkflowOrchestrator` and find the `new` function)

2. **Add parameter to new():**
```rust
pub fn new(
    db: SqlitePool,
    config: Arc<Config>,
    bootstrap_config: &WkmpAiBootstrapConfig,  // Add this parameter
) -> Self {
```

3. **Update memory_monitor initialization:**
```rust
memory_monitor: Arc::new(
    crate::utils::MemoryMonitor::with_threshold(bootstrap_config.memory_usage_threshold_bytes)
),
```

4. **Find where WorkflowOrchestrator is created** (likely in `main.rs` or a startup module)
   - Pass the bootstrap_config through at that point

**Solution B (Alternative): Store in Config**

If the Config struct is more accessible, add memory_usage_threshold_bytes to Config and read from there.

---

### Step 5: Update Main/Startup to Pass Config (30 minutes)

**File:** `wkmp-ai/src/main.rs` (or wherever WorkflowOrchestrator is instantiated)

**Find where WorkflowOrchestrator::new() is called and update:**

```rust
// Assuming bootstrap_config is already available from Stage 1 bootstrap
let orchestrator = WorkflowOrchestrator::new(
    db.clone(),
    config.clone(),
    &bootstrap_config,  // Pass bootstrap config
);
```

---

### Step 6: Fix Tests (20 minutes)

**File:** `wkmp-ai/src/utils/memory_monitor.rs`

**Update test at line 292:**
```rust
#[test]
fn test_custom_threshold() {
    let monitor = MemoryMonitor::with_threshold(100_000_000); // 100MB in bytes
    assert_eq!(monitor.warning_threshold, 100_000_000);
}
```

**Any other tests that create WorkflowOrchestrator:**
- Will need to pass a mock bootstrap_config
- Create a test helper function to generate default bootstrap config

---

## Testing

### Manual Testing
1. Set `ai_memory_usage_threshold_bytes` to a low value (e.g., 1073741824 for 1GB)
2. Run import workflow and verify warnings appear at correct threshold
3. Set to default 12GB and verify no false warnings

### SQL Verification
```sql
-- Check parameter exists with correct default
SELECT key, value FROM settings WHERE key = 'ai_memory_usage_threshold_bytes';
-- Should return: ai_memory_usage_threshold_bytes | 12884901888

-- Test update
UPDATE settings SET value = '8589934592' WHERE key = 'ai_memory_usage_threshold_bytes';
-- Restart wkmp-ai and verify new threshold is used
```

### Log Verification
After implementation, logs should show:
```
Starting memory monitor task with threshold: 12884MB
```

Instead of current:
```
Starting memory monitor task (check interval: 30s)
CRITICAL: Memory usage 3671MB exceeds 2x threshold (1000MB)
```

---

## Gotchas to Avoid

1. **Unit Conversion:** The parameter is in BYTES, but logs show MB. Don't mix units!
   - Database/parameter: bytes (12884901888)
   - Logs/display: MB (memory_bytes / 1_000_000)

2. **Bootstrap Stage:** This is a RESTART_REQUIRED parameter, must be read during bootstrap

3. **Backward Compatibility:** Use COALESCE in SQL to handle databases without the parameter

4. **Validation:** Ensure threshold is reasonable (1GB - 128GB range)

5. **Critical Threshold:** Remains at 2x warning threshold (line 94 of memory_monitor.rs)

---

## Definition of Done

- [ ] Parameter added to database initialization with 12GB default
- [ ] Bootstrap config reads parameter from database
- [ ] Memory monitor uses configured threshold instead of hardcoded 500MB
- [ ] Tests updated and passing
- [ ] Manual test: No false warnings at 3.6GB memory usage
- [ ] Logs show configured threshold at startup

---

## Code References

- **Parameter definition:** docs/IMPL016-settings_reference.md:130-141
- **Memory monitor:** wkmp-ai/src/utils/memory_monitor.rs:52 (hardcoded threshold)
- **Bootstrap config:** wkmp-ai/src/models/bootstrap_config.rs:72-145
- **DB initialization:** wkmp-common/src/db/init.rs:259
- **Orchestrator creation:** wkmp-ai/src/services/workflow_orchestrator/mod.rs:164

---

## Notes

The hardcoded 500MB threshold was appropriate for initial development but causes issues in production. The 12GB default is conservative for modern systems:
- Assumes 16GB+ total RAM
- Leaves 4GB+ for OS and other processes
- Critical threshold at 24GB (2x) prevents runaway memory usage

This change makes wkmp-ai production-ready for real-world music libraries on modern hardware.