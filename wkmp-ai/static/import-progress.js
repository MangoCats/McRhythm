// Import Progress Page JavaScript
// REQ-AIA-UI-001 through REQ-AIA-UI-006: Enhanced multi-level progress display

let eventSource = null;
let currentSessionId = null;
let lastUpdateTime = 0;
const UPDATE_THROTTLE_MS = 100; // REQ-AIA-UI-NF-001: Max 10 updates/sec

// **[AIA-SEC-030]** Validate AcoustID API key before import starts
// Returns true if validation passed or user chose to skip
// Returns false if user cancelled
async function validateAcoustIDBeforeImport() {
    try {
        // Check if API key is configured (with 5 second timeout)
        const controller1 = new AbortController();
        const timeout1 = setTimeout(() => controller1.abort(), 5000);

        const response = await fetch('/api/settings/acoustid_api_key', {
            signal: controller1.signal
        });
        clearTimeout(timeout1);

        if (!response.ok) {
            console.error('Failed to check AcoustID API key');
            return true; // Continue anyway - let pipeline handle it
        }

        const data = await response.json();

        // No API key configured - prompt user
        if (!data.configured) {
            return await promptForAcoustIDKey('No AcoustID API key configured. Please enter a key or skip AcoustID functionality.');
        }

        // API key configured - validate it (with 10 second timeout for external API)
        const controller2 = new AbortController();
        const timeout2 = setTimeout(() => controller2.abort(), 10000);

        const validateResponse = await fetch('/import/validate-acoustid', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ api_key: data.api_key }),
            signal: controller2.signal
        });
        clearTimeout(timeout2);

        if (!validateResponse.ok) {
            console.error('AcoustID validation request failed');
            return true; // Continue anyway - let pipeline handle it
        }

        const validateData = await validateResponse.json();

        if (validateData.valid) {
            console.log('AcoustID API key is valid');
            return true; // Key is valid, proceed
        }

        // Invalid key - prompt user to update or skip
        return await promptForAcoustIDKey(`AcoustID API key is invalid: ${validateData.message}`);

    } catch (error) {
        if (error.name === 'AbortError') {
            console.error('AcoustID validation timed out');
            // Timeout - skip validation and continue
            return true;
        }
        console.error('AcoustID validation failed:', error);
        return true; // Continue anyway - let pipeline handle it
    }
}

// **[AIA-SEC-030]** Prompt user to enter AcoustID API key or skip
// Returns true if user provided valid key or chose to skip
// Returns false if user cancelled
async function promptForAcoustIDKey(errorMessage) {
    return new Promise((resolve) => {
        const modal = document.getElementById('acoustid-modal');
        const errorDisplay = document.getElementById('acoustid-error-message');
        const modalError = document.getElementById('acoustid-modal-error');
        const submitBtn = document.getElementById('acoustid-submit-btn');
        const skipBtn = document.getElementById('acoustid-skip-btn');
        const apiKeyInput = document.getElementById('acoustid-api-key');

        // Show modal with error message
        errorDisplay.textContent = errorMessage;
        modalError.style.display = 'none';
        apiKeyInput.value = '';
        modal.style.display = 'flex';

        // Handle submit - validate and save key
        const handleSubmit = async () => {
            const apiKey = apiKeyInput.value.trim();

            if (!apiKey) {
                modalError.textContent = 'Please enter an API key';
                modalError.style.display = 'block';
                return;
            }

            submitBtn.disabled = true;
            submitBtn.textContent = 'Validating...';
            modalError.style.display = 'none';

            try {
                // Validate the key (with 10 second timeout)
                const controller = new AbortController();
                const timeout = setTimeout(() => controller.abort(), 10000);

                const validateResponse = await fetch('/import/validate-acoustid', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ api_key: apiKey }),
                    signal: controller.signal
                });
                clearTimeout(timeout);

                if (!validateResponse.ok) {
                    throw new Error('Validation request failed');
                }

                const validateData = await validateResponse.json();

                if (!validateData.valid) {
                    // Invalid key - show error and allow retry
                    modalError.textContent = `Invalid API key: ${validateData.message}`;
                    modalError.style.display = 'block';
                    submitBtn.disabled = false;
                    submitBtn.textContent = 'Submit Key';
                    return;
                }

                // Valid key - save it to settings
                const saveResponse = await fetch('/api/settings/acoustid_api_key', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ api_key: apiKey })
                });

                if (!saveResponse.ok) {
                    throw new Error('Failed to save API key');
                }

                // Close modal and proceed
                cleanup();
                modal.style.display = 'none';
                resolve(true);

            } catch (error) {
                console.error('Failed to validate/save AcoustID key:', error);
                if (error.name === 'AbortError') {
                    modalError.textContent = 'Validation timed out. Please check your internet connection and try again.';
                } else {
                    modalError.textContent = error.message || 'Failed to validate API key';
                }
                modalError.style.display = 'block';
                submitBtn.disabled = false;
                submitBtn.textContent = 'Submit Key';
            }
        };

        // Handle skip - proceed without AcoustID
        const handleSkip = () => {
            cleanup();
            modal.style.display = 'none';
            console.log('User chose to skip AcoustID functionality');
            resolve(true);
        };

        // Handle close - cancel import
        const handleClose = () => {
            cleanup();
            modal.style.display = 'none';
            console.log('User cancelled import');
            resolve(false);
        };

        // Cleanup event listeners
        const cleanup = () => {
            submitBtn.removeEventListener('click', handleSubmit);
            skipBtn.removeEventListener('click', handleSkip);
        };

        // Attach event listeners
        submitBtn.addEventListener('click', handleSubmit);
        skipBtn.addEventListener('click', handleSkip);
    });
}

// Start import workflow
async function startImport() {
    const rootFolder = document.getElementById('root-folder').value.trim();
    const startBtn = document.getElementById('start-btn');
    const errorDiv = document.getElementById('error');

    if (!rootFolder) {
        showError('Please enter a root folder path');
        return;
    }

    startBtn.disabled = true;
    startBtn.textContent = 'Validating...';
    errorDiv.style.display = 'none';

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

        const response = await fetch('/import/start', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ root_folder: rootFolder })
        });

        if (!response.ok) {
            const error = await response.json();
            const errorMessage = error?.error?.message || error?.message || 'Failed to start import';
            throw new Error(errorMessage);
        }

        const data = await response.json();
        currentSessionId = data.session_id;

        // Hide setup, show progress sections
        document.getElementById('setup').style.display = 'none';
        document.getElementById('workflow-checklist').style.display = 'block';
        document.getElementById('active-progress').style.display = 'block';
        document.getElementById('current-file').style.display = 'block';
        document.getElementById('time-estimates').style.display = 'flex';

        connectSSE();

    } catch (error) {
        console.error('Import start failed:', error);
        showError(error.message || 'Failed to start import');
        startBtn.disabled = false;
        startBtn.textContent = 'Start Import';
    }
}

// Connection status update
function updateConnectionStatus(status) {
    const statusEl = document.getElementById('connection-status');
    if (statusEl) {
        statusEl.className = 'connection-status status-' + status;
        statusEl.textContent = status === 'connected' ? 'Connected' :
                              status === 'connecting' ? 'Connecting...' : 'Disconnected';
    }
}

// Connect to SSE event stream
function connectSSE() {
    console.log('Connecting to SSE at /import/events');
    updateConnectionStatus('connecting');
    eventSource = new EventSource('/import/events');

    eventSource.onopen = () => {
        console.log('SSE connection opened');
        updateConnectionStatus('connected');
    };

    eventSource.onerror = (err) => {
        console.error('SSE connection error:', err);
        updateConnectionStatus('disconnected');
        // EventSource automatically reconnects
    };

    eventSource.addEventListener('ImportProgressUpdate', (e) => {
        const event = JSON.parse(e.data);

        // REQ-AIA-UI-NF-001: Throttle UI updates
        const now = Date.now();
        if (now - lastUpdateTime < UPDATE_THROTTLE_MS) {
            return; // Skip this update
        }
        lastUpdateTime = now;

        console.log('ImportProgressUpdate:', event);
        updateUI(event);
    });

    eventSource.addEventListener('ImportSessionCompleted', (e) => {
        console.log('ImportSessionCompleted');
        document.getElementById('current-phase-name').textContent = 'Import Completed ✓';
        eventSource.close();
        setTimeout(() => {
            window.location.href = '/import-complete';
        }, 2000);
    });

    eventSource.addEventListener('ImportSessionFailed', (e) => {
        const event = JSON.parse(e.data);
        showError('Import failed: ' + (event.error || 'Unknown error'));
        eventSource.close();
    });
}

// REQ-AIA-UI-001 through REQ-AIA-UI-005: Update all UI sections
function updateUI(event) {
    // **[AIA-SEC-030]** Check for PAUSED state (invalid AcoustID API key)
    if (event.state === 'PAUSED' && event.current_operation &&
        event.current_operation.includes('AcoustID API key invalid')) {
        showAcoustIDKeyModal(event.current_operation);
        return; // Don't update other UI elements while paused
    }

    // REQ-AIA-UI-001: Update workflow checklist
    if (event.phases && event.phases.length > 0) {
        updateWorkflowChecklist(event.phases);
        // Show sub-task status only for active phases with subtasks
        const activePhase = event.phases.find(p => p.status === 'InProgress');
        if (activePhase && activePhase.subtasks && activePhase.subtasks.length > 0) {
            updateSubTaskStatus(activePhase.subtasks);
            document.getElementById('subtask-status').style.display = 'block';
        } else {
            document.getElementById('subtask-status').style.display = 'none';
        }
    }

    // REQ-AIA-UI-002: Update active phase progress
    const percent = event.total > 0 ? Math.round((event.current / event.total) * 100) : 0;
    document.getElementById('current-phase-name').textContent = 'Current Phase: ' + event.state;
    document.getElementById('progress-text').textContent = `${event.current} / ${event.total} files`;
    document.getElementById('progress-percent').textContent = `${percent}%`;
    document.getElementById('progress-bar').style.width = `${percent}%`;
    document.getElementById('progress-bar').textContent = `${percent}%`;

    // REQ-AIA-UI-004: Update current file
    if (event.current_file) {
        const filename = truncateFilename(event.current_file);
        document.getElementById('current-file-path').textContent = filename;
    }

    // REQ-AIA-UI-005: Update time estimates
    document.getElementById('elapsed-time').textContent = formatSeconds(event.elapsed_seconds);
    if (event.estimated_remaining_seconds) {
        document.getElementById('remaining-time').textContent = formatSeconds(event.estimated_remaining_seconds);
    } else {
        document.getElementById('remaining-time').textContent = 'Estimating...';
    }

    // **[PLAN024]** Update phase-specific statistics
    if (event.phase_statistics && event.phase_statistics.length > 0) {
        displayPhaseStatistics(event.phase_statistics);
        document.getElementById('phase-statistics').style.display = 'block';
    }
}

// REQ-AIA-UI-001: Update workflow checklist
function updateWorkflowChecklist(phases) {
    const container = document.getElementById('phases-container');
    container.innerHTML = '';

    phases.forEach(phase => {
        const statusClass = phase.status.toLowerCase().replace(/([A-Z])/g, '-$1').toLowerCase();
        const icon = getPhaseIcon(phase.status);
        const summary = getPhaseSum(phase, phase.status);

        const phaseEl = document.createElement('div');
        phaseEl.className = `phase-item ${statusClass}`;

        // Compact: phase name • description • summary on single line
        const parts = [phase.phase];
        if (phase.description) parts.push(phase.description);
        if (summary) parts.push(summary);
        const compactText = parts.join(' • ');

        phaseEl.innerHTML = `
            <div class="phase-icon">${icon}</div>
            <div class="phase-content">
                <div class="phase-name">${compactText}</div>
            </div>
        `;
        container.appendChild(phaseEl);
    });
}

// REQ-AIA-UI-003: Update sub-task status
function updateSubTaskStatus(subtasks) {
    const container = document.getElementById('subtasks-container');
    container.innerHTML = '';

    subtasks.forEach(subtask => {
        const total = subtask.success_count + subtask.failure_count;
        const successRate = total > 0 ? (subtask.success_count / total * 100).toFixed(1) : 0;
        const colorClass = getColorClass(parseFloat(successRate));

        const subtaskEl = document.createElement('div');
        subtaskEl.className = `subtask-item ${colorClass}`;
        subtaskEl.innerHTML = `
            <div>
                <div class="subtask-name">${subtask.name}</div>
                <div class="subtask-stats">${subtask.success_count} success, ${subtask.failure_count} failed</div>
            </div>
            <div style="font-weight: bold;">${successRate}% ${getStatusIcon(colorClass)}</div>
        `;
        container.appendChild(subtaskEl);
    });
}

// **[PLAN024]** Display phase-specific statistics
function displayPhaseStatistics(statistics) {
    const container = document.getElementById('phase-statistics-container');
    if (!container) return;

    container.innerHTML = '';

    statistics.forEach(stat => {
        const statEl = document.createElement('div');
        statEl.className = 'phase-stat-item';

        let content = '';
        const phaseName = stat.phase_name;

        // Format statistics based on phase type (per wkmp-ai_refinement.md lines 74-103)
        switch (phaseName) {
            case 'SCANNING':
                content = stat.is_scanning
                    ? 'scanning'
                    : `${stat.potential_files_found} potential files found`;
                break;

            case 'PROCESSING':
                content = `Processing ${stat.completed} to ${stat.started} of ${stat.total}`;
                break;

            case 'FILENAME_MATCHING':
                content = `${stat.completed_filenames_found} completed filenames found`;
                break;

            case 'HASHING':
                content = `${stat.hashes_computed} hashes computed, ${stat.matches_found} matches found`;
                break;

            case 'EXTRACTING':
                content = `Metadata successfully extracted from ${stat.successful_extractions} files, ${stat.failures} failures`;
                break;

            case 'SEGMENTING':
                content = `${stat.files_processed} files, ${stat.potential_passages} potential passages, ${stat.finalized_passages} finalized passages, ${stat.songs_identified} songs identified`;
                break;

            case 'FINGERPRINTING':
                content = `${stat.passages_fingerprinted} potential passages fingerprinted, ${stat.successful_matches} successfully matched`;
                break;

            case 'SONG_MATCHING':
                content = `${stat.high_confidence} high, ${stat.medium_confidence} medium, ${stat.low_confidence} low, ${stat.no_confidence} no confidence`;
                break;

            case 'RECORDING':
                // Scrollable list of recorded passages
                if (stat.recorded_passages && stat.recorded_passages.length > 0) {
                    const list = stat.recorded_passages.map(p => {
                        const title = p.song_title || 'unidentified passage';
                        return `<div class="passage-item">${title} in ${p.file_path}</div>`;
                    }).join('');
                    content = `<div class="scrollable-list">${list}</div>`;
                } else {
                    content = 'No passages recorded yet';
                }
                break;

            case 'AMPLITUDE':
                // Scrollable list of analyzed passages with timing
                if (stat.analyzed_passages && stat.analyzed_passages.length > 0) {
                    const list = stat.analyzed_passages.map(p => {
                        const title = p.song_title || 'unidentified passage';
                        return `<div class="passage-item">${title} ${p.passage_length_seconds.toFixed(1)}s lead-in ${p.lead_in_ms} ms lead-out ${p.lead_out_ms} ms</div>`;
                    }).join('');
                    content = `<div class="scrollable-list">${list}</div>`;
                } else {
                    content = 'No passages analyzed yet';
                }
                break;

            case 'FLAVORING':
                content = `${stat.pre_existing} pre-existing, ${stat.acousticbrainz} by AcousticBrainz, ${stat.essentia} by Essentia, ${stat.failed} could not be flavored`;
                break;

            case 'PASSAGES_COMPLETE':
                content = `${stat.passages_completed} passages completed`;
                break;

            case 'FILES_COMPLETE':
                content = `${stat.files_completed} files completed`;
                break;

            default:
                content = JSON.stringify(stat);
        }

        statEl.innerHTML = `
            <div class="phase-stat-name">${phaseName}</div>
            <div class="phase-stat-content">${content}</div>
        `;
        container.appendChild(statEl);
    });
}

// Helper functions
function getPhaseIcon(status) {
    const icons = {
        'Pending': '○',
        'InProgress': '⟳',
        'Completed': '✓',
        'Failed': '✗',
        'CompletedWithWarnings': '⚠'
    };
    return icons[status] || '○';
}

function getPhaseSum(phase, status) {
    if (status === 'Completed' || status === 'CompletedWithWarnings') {
        return `Completed - ${phase.progress_current}/${phase.progress_total} processed`;
    } else if (status === 'InProgress') {
        return `In Progress - ${phase.progress_current}/${phase.progress_total} processed`;
    } else if (status === 'Pending') {
        return 'Pending';
    }
    return '';
}

function getColorClass(successRate) {
    if (successRate > 95) return 'green';
    if (successRate >= 85) return 'yellow';
    return 'red';
}

function getStatusIcon(colorClass) {
    if (colorClass === 'green') return '✓';
    if (colorClass === 'yellow') return '⚠';
    return '✗';
}

// REQ-AIA-UI-004: Truncate filename if >80 chars (show basename)
function truncateFilename(path) {
    if (path.length <= 80) return path;
    const parts = path.split('/');
    return parts[parts.length - 1];
}

// REQ-AIA-UI-005: Format seconds to human-readable
function formatSeconds(seconds) {
    if (!seconds) return '0s';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) return `${h}h ${m}m ${s}s`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
}

function showError(message) {
    const errorDiv = document.getElementById('error');
    errorDiv.textContent = message;
    errorDiv.style.display = 'block';
}

// **[AIA-SEC-030]** AcoustID API key validation modal
function showAcoustIDKeyModal(errorMessage) {
    const modal = document.getElementById('acoustid-modal');
    const errorDisplay = document.getElementById('acoustid-error-message');

    if (modal && errorDisplay) {
        errorDisplay.textContent = errorMessage;
        modal.style.display = 'flex';
    }
}

function closeAcoustIDModal() {
    const modal = document.getElementById('acoustid-modal');
    if (modal) {
        modal.style.display = 'none';
    }
}

async function submitAcoustIDKey() {
    const apiKey = document.getElementById('acoustid-api-key').value.trim();
    const submitBtn = document.getElementById('acoustid-submit-btn');
    const errorDisplay = document.getElementById('acoustid-modal-error');

    if (!apiKey) {
        errorDisplay.textContent = 'Please enter an API key';
        errorDisplay.style.display = 'block';
        return;
    }

    submitBtn.disabled = true;
    submitBtn.textContent = 'Validating...';
    errorDisplay.style.display = 'none';

    try {
        const response = await fetch('/import/acoustid-key', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                session_id: currentSessionId,
                api_key: apiKey
            })
        });

        if (!response.ok) {
            const error = await response.json();
            throw new Error(error?.error?.message || 'Failed to validate API key');
        }

        // Close modal and resume import
        closeAcoustIDModal();
        console.log('AcoustID API key updated successfully');

    } catch (error) {
        console.error('Failed to update AcoustID key:', error);
        errorDisplay.textContent = error.message || 'Failed to validate API key';
        errorDisplay.style.display = 'block';
        submitBtn.disabled = false;
        submitBtn.textContent = 'Submit Key';
    }
}

async function skipAcoustID() {
    const skipBtn = document.getElementById('acoustid-skip-btn');
    const errorDisplay = document.getElementById('acoustid-modal-error');

    skipBtn.disabled = true;
    skipBtn.textContent = 'Skipping...';
    errorDisplay.style.display = 'none';

    try {
        const response = await fetch('/import/acoustid-skip', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ session_id: currentSessionId })
        });

        if (!response.ok) {
            const error = await response.json();
            throw new Error(error?.error?.message || 'Failed to skip AcoustID');
        }

        // Close modal and resume import
        closeAcoustIDModal();
        console.log('AcoustID skipped successfully');

    } catch (error) {
        console.error('Failed to skip AcoustID:', error);
        errorDisplay.textContent = error.message || 'Failed to skip AcoustID';
        errorDisplay.style.display = 'block';
        skipBtn.disabled = false;
        skipBtn.textContent = 'Skip AcoustID';
    }
}

console.log('Enhanced import progress page loaded (PLAN011)');

// **[AIA-SEC-030]** Check for active import session on page load
// If import is in progress, restore the progress UI instead of showing setup
async function checkForActiveSession() {
    try {
        const response = await fetch('/import/active');
        if (!response.ok) {
            console.log('No active import session');
            return;
        }

        const data = await response.json();
        if (data && data.session_id) {
            console.log('Active import session found:', data.session_id);
            currentSessionId = data.session_id;

            // Hide setup, show progress sections
            document.getElementById('setup').style.display = 'none';
            document.getElementById('workflow-checklist').style.display = 'block';
            document.getElementById('active-progress').style.display = 'block';
            document.getElementById('current-file').style.display = 'block';
            document.getElementById('time-estimates').style.display = 'flex';

            // Update UI with current progress
            updateUI({
                session_id: data.session_id,
                state: data.state,
                current: data.progress.progress_current,
                total: data.progress.progress_total,
                current_operation: data.current_operation,
                elapsed_seconds: data.elapsed_seconds,
                estimated_remaining_seconds: data.estimated_remaining_seconds,
                phases: data.progress.phases || [],
                current_file: data.progress.current_file
            });

            // Connect to SSE to receive ongoing updates
            connectSSE();
        }
    } catch (error) {
        console.error('Failed to check for active session:', error);
    }
}

// Check for active session when page loads
checkForActiveSession();

// Connect to general SSE for connection status monitoring on page load
// This is separate from the import-specific SSE that connects when "Start Import" is clicked
// Note: WkmpSSEConnection class is loaded from /static/wkmp-sse.js in the HTML
const generalSSE = new WkmpSSEConnection('/events', 'connection-status');
generalSSE.connect();
