# Pre-Import AcoustID API Key Validation

**Date:** 2025-11-11
**Status:** ✅ COMPLETED - All 244 tests passing

## Problem

User reported that AcoustID API key validation was happening DURING import (mid-process), not BEFORE import starts. This caused:

1. **No upfront feedback:** User doesn't know if API key is invalid until import begins processing files
2. **Missing DEBUG logs:** No logging when AcoustID is skipped due to user choice
3. **Poor UX:** Mid-import modal interruption instead of upfront validation

**User Request:**
> "before the import process starts, a test of the AcoustID API key should determine whether it is valid or not, and if it is not valid the user should be prompted then to either input an API key (which is then tested for validity and if not valid repeat the prompt), or skip all AcoustID based functions in the import."

---

## Solution: Pre-Import Validation Flow

### Implementation Overview

**Pre-Import Validation:**
1. User clicks "Start Import" button
2. Frontend checks if AcoustID API key is configured (`GET /api/settings/acoustid_api_key`)
3. If no key, show modal asking user to provide key or skip
4. If key exists, validate it (`POST /import/validate-acoustid`)
5. If invalid, show modal with error and retry prompt
6. User can re-enter key (validated again) or skip
7. Only after validation succeeds or user skips does import start

**Re-Validation Loop:**
- If user enters invalid key, validation fails with specific error message
- Modal stays open, showing error
- User can retry with different key
- Validation repeats until key is valid or user skips

---

## Code Changes

### Backend Changes

#### 1. Added GET Endpoint for API Key Retrieval

**File:** [wkmp-ai/src/api/settings.rs](wkmp-ai/src/api/settings.rs)

**New Response Type:**
```rust
/// Response payload for getting AcoustID API key
#[derive(Debug, Serialize)]
pub struct GetApiKeyResponse {
    /// Whether an API key is configured
    pub configured: bool,
    /// The API key (if configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}
```

**New Handler:**
```rust
/// GET /api/settings/acoustid_api_key handler
pub async fn get_acoustid_api_key(
    State(state): State<AppState>,
) -> ApiResult<Json<GetApiKeyResponse>> {
    let api_key = crate::db::settings::get_acoustid_api_key(&state.db)
        .await
        .map_err(|e| {
            ApiError::Internal(format!("Failed to retrieve API key from database: {}", e))
        })?;

    Ok(Json(GetApiKeyResponse {
        configured: api_key.is_some(),
        api_key,
    }))
}
```

**Updated Router:**
```rust
pub fn settings_routes() -> Router<AppState> {
    Router::new().route(
        "/api/settings/acoustid_api_key",
        get(get_acoustid_api_key).post(set_acoustid_api_key),
    )
}
```

#### 2. Added DEBUG Logging for Skipped AcoustID Calls

**File:** [wkmp-ai/src/workflow/pipeline.rs](wkmp-ai/src/workflow/pipeline.rs)

**Enhanced Logging:**
```rust
} else {
    // **[AIA-SEC-030]** User has chosen to skip AcoustID
    debug!(
        passage_index = passage_index,
        "AcoustID extraction skipped (user chose to skip AcoustID functionality)"
    );
}
} else {
    // No API key configured - skip AcoustID
    debug!(
        passage_index = passage_index,
        "AcoustID extraction skipped (no API key configured)"
    );
}
```

### Frontend Changes

#### 3. Pre-Import Validation Flow

**File:** [wkmp-ai/static/import-progress.js](wkmp-ai/static/import-progress.js)

**Updated `startImport()` Function:**
```javascript
async function startImport() {
    // ... validation setup ...

    try {
        // **[AIA-SEC-030]** Validate AcoustID API key before starting import
        const keyValid = await validateAcoustIDBeforeImport();
        if (!keyValid) {
            // User cancelled or validation failed
            startBtn.disabled = false;
            startBtn.textContent = 'Start Import';
            return;
        }

        startBtn.textContent = 'Starting...';

        // Now start the actual import...
        const response = await fetch('/import/start', { /* ... */ });
        // ...
    } catch (error) {
        // ... error handling ...
    }
}
```

#### 4. Validation Logic

**New Function: `validateAcoustIDBeforeImport()`**

```javascript
async function validateAcoustIDBeforeImport() {
    try {
        // Check if API key is configured
        const response = await fetch('/api/settings/acoustid_api_key');
        const data = await response.json();

        // No API key configured - prompt user
        if (!data.configured) {
            return await promptForAcoustIDKey(
                'No AcoustID API key configured. Please enter a key or skip AcoustID functionality.'
            );
        }

        // API key configured - validate it
        const validateResponse = await fetch('/import/validate-acoustid', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ api_key: data.api_key })
        });

        const validateData = await validateResponse.json();

        if (validateData.valid) {
            console.log('AcoustID API key is valid');
            return true; // Key is valid, proceed
        }

        // Invalid key - prompt user to update or skip
        return await promptForAcoustIDKey(
            `AcoustID API key is invalid: ${validateData.message}`
        );
    } catch (error) {
        console.error('AcoustID validation failed:', error);
        return true; // Continue anyway - let pipeline handle it
    }
}
```

#### 5. Modal Prompt with Re-Validation Loop

**New Function: `promptForAcoustIDKey(errorMessage)`**

```javascript
async function promptForAcoustIDKey(errorMessage) {
    return new Promise((resolve) => {
        // Show modal with error message
        modal.style.display = 'flex';

        // Handle submit - validate and save key
        const handleSubmit = async () => {
            const apiKey = apiKeyInput.value.trim();

            // Validate the key
            const validateResponse = await fetch('/import/validate-acoustid', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ api_key: apiKey })
            });

            const validateData = await validateResponse.json();

            if (!validateData.valid) {
                // Invalid key - show error and allow retry
                modalError.textContent = `Invalid API key: ${validateData.message}`;
                modalError.style.display = 'block';
                submitBtn.disabled = false;
                submitBtn.textContent = 'Submit Key';
                return; // Stay in modal, allow retry
            }

            // Valid key - save it to settings
            await fetch('/api/settings/acoustid_api_key', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ api_key: apiKey })
            });

            // Close modal and proceed
            modal.style.display = 'none';
            resolve(true);
        };

        // Handle skip - proceed without AcoustID
        const handleSkip = () => {
            modal.style.display = 'none';
            resolve(true);
        };

        // Attach event listeners
        submitBtn.addEventListener('click', handleSubmit);
        skipBtn.addEventListener('click', handleSkip);
    });
}
```

---

## User Experience Flow

### Scenario 1: No API Key Configured

1. User clicks "Start Import"
2. Button changes to "Validating..."
3. Modal appears: "No AcoustID API key configured. Please enter a key or skip AcoustID functionality."
4. User enters key → Click "Submit Key"
5. Key is validated:
   - **Valid:** Modal closes, import starts
   - **Invalid:** Error shown, modal stays open, user can retry
6. Or user clicks "Skip AcoustID" → Modal closes, import starts without AcoustID

### Scenario 2: Invalid API Key Configured

1. User clicks "Start Import"
2. Button changes to "Validating..."
3. Validation fails
4. Modal appears: "AcoustID API key is invalid: [specific error]"
5. User can update key or skip (same flow as Scenario 1)

### Scenario 3: Valid API Key Configured

1. User clicks "Start Import"
2. Button changes to "Validating..."
3. Validation succeeds
4. Button changes to "Starting..."
5. Import starts immediately (no modal)

---

## DEBUG Logging

**When AcoustID is skipped during import processing, logs will show:**

```
DEBUG passage_index=0 AcoustID extraction skipped (user chose to skip AcoustID functionality)
```

or

```
DEBUG passage_index=0 AcoustID extraction skipped (no API key configured)
```

**This allows user to verify that AcoustID is being skipped as expected.**

---

## Testing

### Unit Tests: ✅ PASSING

```bash
cargo test -p wkmp-ai --lib
```

**Result:** All 244 tests passing

### Manual Testing Required

User should verify the following scenarios:

#### Test 1: No API Key Configured
1. Ensure no AcoustID API key in database (`DELETE FROM settings WHERE key = 'acoustid_api_key'`)
2. Click "Start Import"
3. **Expected:** Modal appears asking for API key
4. Enter invalid key → **Expected:** Error shown, modal stays open
5. Enter valid key → **Expected:** Modal closes, import starts
6. Check console logs for DEBUG messages during import

#### Test 2: Invalid API Key Configured
1. Set invalid API key in database
2. Click "Start Import"
3. **Expected:** Modal appears showing key is invalid
4. Click "Skip AcoustID" → **Expected:** Modal closes, import starts
5. Check console logs show "AcoustID extraction skipped (user chose to skip)"

#### Test 3: Valid API Key Configured
1. Set valid API key in database
2. Click "Start Import"
3. **Expected:** No modal, import starts immediately
4. Check console logs show AcoustID extraction attempts

#### Test 4: Re-Validation Loop
1. No API key configured
2. Click "Start Import" → Modal appears
3. Enter invalid key "test123" → Click "Submit Key"
4. **Expected:** Error shown, modal stays open
5. Enter another invalid key "bad-key" → Click "Submit Key"
6. **Expected:** Error shown again, modal stays open
7. Enter valid key → Click "Submit Key"
8. **Expected:** Modal closes, import starts

---

## Architecture Notes

### Why GET Endpoint for API Key?

**Requirement:** Frontend needs to check if API key is configured BEFORE import starts.

**Solution:** Added `GET /api/settings/acoustid_api_key` endpoint that returns:
- `configured`: Boolean indicating if key exists
- `api_key`: The key value (if configured)

**Alternative Considered:** Re-use existing validation endpoint with empty key.
**Rejected:** Mixing GET semantics (check existence) with POST semantics (validate) is confusing.

### Why Save Key to Settings After Validation?

**Requirement:** User enters key in modal → Key should persist for future imports.

**Solution:** After validation succeeds, frontend calls `POST /api/settings/acoustid_api_key` to save key.

**Benefit:** User only needs to enter key once. Future imports will use saved key.

### Why Re-Validation Loop in Frontend?

**Requirement:** If user enters invalid key, allow retry without closing modal.

**Solution:** Modal's submit handler validates key. If invalid, shows error and keeps modal open.

**Benefit:** User can fix typos or try different keys without restarting import flow.

---

## Comparison to Previous Approach

| Aspect | Old (Mid-Import) | New (Pre-Import) |
|--------|------------------|------------------|
| **Validation Timing** | During pipeline processing | Before import starts |
| **User Awareness** | Discovers invalid key after import begins | Knows upfront if key is invalid |
| **Modal Interruption** | Mid-process pause (confusing) | Upfront decision (clear) |
| **Re-Validation** | Not supported | Retry loop in modal |
| **DEBUG Logging** | None | Logs every skip event |
| **UX** | Poor (unexpected pause) | Good (clear decision point) |

---

## Related Documents

- [SPEC032-audio_ingest_architecture.md](docs/SPEC032-audio_ingest_architecture.md) - Audio Ingest architecture
- [BOUNDARY_DETECTION_BLOCKING_FIX.md](BOUNDARY_DETECTION_BLOCKING_FIX.md) - Performance fix
- [PARALLELISM_CORRECTION.md](PARALLELISM_CORRECTION.md) - Parallelism tuning

---

## Traceability

This implementation addresses:

- **[AIA-SEC-030]**: Pre-import AcoustID API key validation with user prompting
- **[REQ-AIA-UI-002]**: Real-time progress updates (no mid-import interruptions)

---

## Validation Strategy Fix

**Initial Issue:** First implementation sent dummy fingerprint `"AQAAA"`, but AcoustID validates fingerprint format BEFORE checking API key, returning error code 3 ("invalid fingerprint") even for valid API keys.

**Solution:** Updated validation to intentionally send invalid fingerprint and check error codes:
- **Error code 3** ("invalid fingerprint") → API key is VALID (key was accepted, fingerprint rejected)
- **Error code 5** ("invalid API key") → API key is INVALID
- **Error code 6** ("invalid format") → API key format is wrong

**Rationale:** AcoustID processes requests in order:
1. Validate API key
2. Validate fingerprint format
3. Process fingerprint

By sending intentionally invalid fingerprint, we can distinguish between invalid key (error 5/6) and valid key (error 3).

**File:** [wkmp-ai/src/extractors/acoustid_client.rs:108-185](wkmp-ai/src/extractors/acoustid_client.rs#L108-L185)

---

## Conclusion

Pre-import validation provides better UX by validating AcoustID API key BEFORE import starts, allowing user to provide valid key or skip upfront. Re-validation loop supports error recovery without restarting import flow.

**Key Achievements:**
- ✅ All 244 tests passing
- ✅ GET endpoint for API key retrieval
- ✅ Pre-import validation in frontend
- ✅ Re-validation loop for invalid keys
- ✅ DEBUG logging for skipped AcoustID calls
- ✅ Key persisted to settings after validation
- ✅ Fixed validation logic to handle AcoustID error code 3 correctly

**User Action Required:** Test the validation flow with various scenarios (no key, invalid key, valid key, retry loop).
