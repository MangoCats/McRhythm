# TC-U-SAFE-010-01: Settings Backup and Restore

**Requirement:** TUNE-SAFE-010 (lines 371-374)
**Test Type:** Unit Test
**Priority:** High
**Estimated Effort:** 20 minutes

---

## Test Objective

Verify that user settings are backed up before tuning and correctly restored on abort.

---

## Test Specification

### Given: Database with current mixer and buffer settings

```rust
// Initial settings in database
mixer_check_interval_ms = 5
audio_buffer_size = 512
```

### When: Tuning starts, settings are backed up, then abort is simulated

```rust
// 1. Backup settings
let backup = backup_settings(&db).await?;
assert_eq!(backup.mixer_check_interval_ms, 5);
assert_eq!(backup.audio_buffer_size, 512);

// 2. Modify settings (simulate tuning in progress)
set_setting(&db, "mixer_check_interval_ms", 20).await?;
set_setting(&db, "audio_buffer_size", 1024).await?;

// 3. Simulate abort and restore
restore_settings(&db, &backup).await?;
```

### Then: Original settings are restored

```rust
let restored_interval = load_clamped_setting(&db, "mixer_check_interval_ms", 1, 100, 5).await?;
let restored_buffer = load_clamped_setting(&db, "audio_buffer_size", 64, 8192, 512).await?;

assert_eq!(restored_interval, 5, "Interval should be restored to original");
assert_eq!(restored_buffer, 512, "Buffer should be restored to original");
```

---

## Verify

### Assertions

```rust
#[tokio::test]
async fn test_settings_backup_restore() {
    let db = setup_test_db().await;

    // Set initial values
    set_setting(&db, "mixer_check_interval_ms", 5u64).await.unwrap();
    set_setting(&db, "audio_buffer_size", 512u32).await.unwrap();

    // Backup
    let backup = SettingsBackup {
        mixer_check_interval_ms: 5,
        audio_buffer_size: 512,
        timestamp: Utc::now(),
    };

    // Write backup to temp file
    write_backup_file(&backup).unwrap();

    // Modify settings
    set_setting(&db, "mixer_check_interval_ms", 20u64).await.unwrap();
    set_setting(&db, "audio_buffer_size", 1024u32).await.unwrap();

    // Restore from backup
    restore_settings(&db, &backup).await.unwrap();

    // Verify restoration
    let interval = load_clamped_setting(&db, "mixer_check_interval_ms", 1, 100, 5).await.unwrap();
    let buffer = load_clamped_setting(&db, "audio_buffer_size", 64, 8192, 512).await.unwrap();

    assert_eq!(interval, 5);
    assert_eq!(buffer, 512);
}
```

### Pass Criteria

- ✓ Backup captures current settings correctly
- ✓ Backup written to temp file (std::env::temp_dir()/wkmp_tuning_backup.json)
- ✓ Restore overwrites modified settings with backed-up values
- ✓ Temp file cleaned up on successful completion

### Fail Criteria

- ✗ Backup fails to capture settings
- ✗ Restore doesn't overwrite modified settings
- ✗ Temp file not created or not accessible
- ✗ Restore corrupts database

---

## Edge Cases

### Edge Case 1: Settings don't exist (use defaults)

```rust
// Database empty, no settings
let backup = backup_settings(&db).await?;
assert_eq!(backup.mixer_check_interval_ms, 5); // Default
assert_eq!(backup.audio_buffer_size, 512); // Default
```

### Edge Case 2: Temp file write fails (permission denied)

```rust
// Mock filesystem error
let result = write_backup_file(&backup);
assert!(result.is_err(), "Should handle file write failure");

// Should still maintain in-memory backup
let restored = restore_from_memory(&backup);
assert!(restored.is_ok());
```

### Edge Case 3: Restore during panic

```rust
#[tokio::test]
#[should_panic(expected = "simulated panic")]
async fn test_restore_on_panic() {
    let db = setup_test_db().await;
    let backup = backup_settings(&db).await.unwrap();

    // Setup panic hook to restore settings
    let restore_on_panic = move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            restore_settings(&db, &backup).await.unwrap();
        });
    };

    std::panic::set_hook(Box::new(move |_| {
        restore_on_panic();
    }));

    panic!("simulated panic");
}
```

---

## Test Data

**Backup Structure:**
```rust
struct SettingsBackup {
    mixer_check_interval_ms: u64,
    audio_buffer_size: u32,
    timestamp: DateTime<Utc>,
}
```

**Temp File Location:**
- Path: `std::env::temp_dir() / "wkmp_tuning_backup.json"`
- Format: JSON
- Example:
```json
{
  "mixer_check_interval_ms": 5,
  "audio_buffer_size": 512,
  "timestamp": "2025-10-26T22:00:00Z"
}
```

---

## Implementation Notes

```rust
pub async fn backup_settings(db: &Pool<Sqlite>) -> Result<SettingsBackup> {
    let mixer_check_interval_ms = load_clamped_setting(
        db, "mixer_check_interval_ms", 1, 100, 5
    ).await?;

    let audio_buffer_size = load_clamped_setting(
        db, "audio_buffer_size", 64, 8192, 512
    ).await?;

    let backup = SettingsBackup {
        mixer_check_interval_ms,
        audio_buffer_size,
        timestamp: Utc::now(),
    };

    // Write to temp file as safety measure
    write_backup_file(&backup)?;

    Ok(backup)
}

pub async fn restore_settings(db: &Pool<Sqlite>, backup: &SettingsBackup) -> Result<()> {
    set_setting(db, "mixer_check_interval_ms", backup.mixer_check_interval_ms).await?;
    set_setting(db, "audio_buffer_size", backup.audio_buffer_size).await?;

    // Clean up temp file
    cleanup_backup_file();

    Ok(())
}

fn write_backup_file(backup: &SettingsBackup) -> Result<()> {
    let path = std::env::temp_dir().join("wkmp_tuning_backup.json");
    let json = serde_json::to_string_pretty(backup)?;
    std::fs::write(path, json)?;
    Ok(())
}
```

---

## Traceability

**Requirement:** TUNE-SAFE-010
**Related Tests:** TC-U-SAFE-010-02 (restore on panic), TC-U-SRC-040-02 (abort handling)
**Validates:** Settings preservation and restore functionality
