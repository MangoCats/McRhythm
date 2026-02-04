// Import Progress Page JavaScript
// REQ-AIA-UI-001 through REQ-AIA-UI-006: Enhanced multi-level progress display

let eventSource = null;
let currentSessionId = null;
let lastUpdateTime = 0;
const UPDATE_THROTTLE_MS = 100; // REQ-AIA-UI-NF-001: Max 10 updates/sec

// **[AIA-UI-010]** Worker activity live update tracking
let currentWorkerActivities = [];
let workerUpdateInterval = null;

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

        // **[AIA-UI-010]** Stop worker live updates
        if (workerUpdateInterval) {
            clearInterval(workerUpdateInterval);
            workerUpdateInterval = null;
        }
        currentWorkerActivities = [];

        eventSource.close();
        setTimeout(() => {
            window.location.href = '/import-complete';
        }, 2000);
    });

    eventSource.addEventListener('ImportSessionFailed', (e) => {
        const event = JSON.parse(e.data);
        showError('Import failed: ' + (event.error || 'Unknown error'));

        // **[AIA-UI-010]** Stop worker live updates
        if (workerUpdateInterval) {
            clearInterval(workerUpdateInterval);
            workerUpdateInterval = null;
        }
        currentWorkerActivities = [];

        eventSource.close();
    });

    // **[PLAN032]** Handle analysis log events
    eventSource.addEventListener('AnalysisLog', (e) => {
        const event = JSON.parse(e.data);
        handleAnalysisLog(event);

        // Show log panel on first entry
        const panel = document.getElementById('analysis-log');
        if (panel && panel.style.display === 'none') {
            panel.style.display = 'block';
        }
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
        updateWorkflowChecklist(event.phases, event.phase_statistics);
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

    // Hide progress bar during SCANNING (we don't know total file count yet)
    const progressBarEl = document.getElementById('progress-bar');
    const progressTextEl = document.getElementById('progress-text');
    const progressPercentEl = document.getElementById('progress-percent');

    if (event.state === 'Scanning') {
        // Hide progress bar and percentage during scanning
        if (progressBarEl && progressBarEl.parentElement) {
            progressBarEl.parentElement.style.display = 'none';
        }
        if (progressPercentEl) progressPercentEl.style.display = 'none';

        // Show live file counts during scanning
        if (progressTextEl && event.phase_statistics && event.phase_statistics.length > 0) {
            const scanStat = event.phase_statistics.find(s => s.phase_name === 'SCANNING');
            if (scanStat) {
                const total = scanStat.audio_files + scanStat.image_files + scanStat.other_files;
                progressTextEl.style.display = '';
                progressTextEl.textContent = `${total} files found (${scanStat.audio_files} audio, ${scanStat.image_files} image, ${scanStat.other_files} other)`;
            } else {
                progressTextEl.style.display = 'none';
            }
        } else if (progressTextEl) {
            progressTextEl.style.display = 'none';
        }
    } else {
        // Show progress indicators during other phases
        if (progressBarEl && progressBarEl.parentElement) {
            progressBarEl.parentElement.style.display = '';
        }
        if (progressTextEl) {
            progressTextEl.style.display = '';
            progressTextEl.textContent = `${event.current} / ${event.total} files`;
        }
        if (progressPercentEl) {
            progressPercentEl.style.display = '';
            progressPercentEl.textContent = `${percent}%`;
        }
        progressBarEl.style.width = `${percent}%`;
        progressBarEl.textContent = `${percent}%`;
    }

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
function updateWorkflowChecklist(phases, phaseStatistics) {
    const container = document.getElementById('phases-container');
    container.innerHTML = '';

    phases.forEach(phase => {
        const statusClass = phase.status.toLowerCase().replace(/([A-Z])/g, '-$1').toLowerCase();
        const icon = getPhaseIcon(phase.status);
        const summary = getPhaseSum(phase, phase.status, phaseStatistics);

        const phaseEl = document.createElement('div');
        phaseEl.className = `phase-item ${statusClass}`;

        // Compact: phase name • description • summary on single line
        const parts = [phase.phase];
        // Skip description for completed SCANNING phase (summary contains file counts)
        const isCompletedScanning = phase.phase === 'SCANNING' &&
            (phase.status === 'Completed' || phase.status === 'CompletedWithWarnings');
        if (phase.description && !isCompletedScanning) parts.push(phase.description);
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
                // While scanning: show magic byte analysis progress
                if (stat.is_scanning) {
                    if (stat.total_files > 0 && stat.magic_byte_analyzed > 0) {
                        const percentage = ((stat.magic_byte_analyzed / stat.total_files) * 100).toFixed(1);
                        content = `Analyzing files: ${stat.magic_byte_analyzed} of ${stat.total_files} (${percentage}%)`;
                    } else {
                        content = 'Discovering files...';
                    }
                } else {
                    // After completion: show file type breakdown with verification details
                    const total = stat.audio_files + stat.image_files + stat.other_files;
                    let breakdown = `Found: ${stat.audio_files} audio files, ${stat.image_files} image files, ${stat.other_files} other files, ${total} Total`;

                    // Add verification summary if available
                    if (stat.audio_confirmed !== undefined) {
                        const verificationDetails = [];
                        if (stat.audio_confirmed > 0) verificationDetails.push(`${stat.audio_confirmed} confirmed audio`);
                        if (stat.image_confirmed > 0) verificationDetails.push(`${stat.image_confirmed} confirmed image`);
                        if (stat.misleading_extension > 0) verificationDetails.push(`${stat.misleading_extension} misleading ext`);
                        if (stat.audio_unrecognized_ext > 0) verificationDetails.push(`${stat.audio_unrecognized_ext} unrecognized audio ext`);
                        if (stat.image_unrecognized_ext > 0) verificationDetails.push(`${stat.image_unrecognized_ext} unrecognized image ext`);

                        if (verificationDetails.length > 0) {
                            breakdown += ` (${verificationDetails.join(', ')})`;
                        }
                    }

                    content = breakdown;
                }
                break;

            case 'PROCESSING':
                // **[AIA-UI-010]** Display worker activity tracking
                let workerSection = '';
                if (stat.workers && stat.workers.length > 0) {
                    // Store current workers for live update interval
                    currentWorkerActivities = stat.workers;

                    const workerList = stat.workers.map(w => {
                        const fileDisplay = w.file_path || 'idle';
                        const phaseDisplay = w.phase_name || 'waiting';

                        // Calculate elapsed time client-side from phase_started_at
                        let elapsedDisplay = '';
                        if (w.phase_started_at) {
                            const startTime = new Date(w.phase_started_at);
                            const elapsedSeconds = (Date.now() - startTime.getTime()) / 1000;
                            elapsedDisplay = `Started ${elapsedSeconds.toFixed(1)} seconds ago.`;
                        }

                        // Add passage timing if present
                        let passageDisplay = '';
                        if (w.passage_start_seconds !== null && w.passage_end_seconds !== null) {
                            const startMin = Math.floor(w.passage_start_seconds / 60);
                            const startSec = (w.passage_start_seconds % 60).toFixed(0);
                            const endMin = Math.floor(w.passage_end_seconds / 60);
                            const endSec = (w.passage_end_seconds % 60).toFixed(0);
                            passageDisplay = ` [${startMin}:${startSec.padStart(2, '0')}-${endMin}:${endSec.padStart(2, '0')}]`;
                        }

                        return `<div class="worker-item" data-worker-id="${w.worker_id}">Worker ${w.worker_id}: ${phaseDisplay} - ${fileDisplay}${passageDisplay} <span class="worker-elapsed">${elapsedDisplay}</span></div>`;
                    }).join('');
                    workerSection = `<div class="scrollable-list worker-list">${workerList}</div>`;

                    // Start live update interval if not already running
                    if (!workerUpdateInterval) {
                        startWorkerLiveUpdates();
                    }
                } else {
                    // No workers, clear stored activities and stop interval
                    currentWorkerActivities = [];
                    if (workerUpdateInterval) {
                        clearInterval(workerUpdateInterval);
                        workerUpdateInterval = null;
                    }
                }

                // **[File Processing Status List]** Display file processing history
                let fileSection = '';
                if (stat.files && stat.files.length > 0) {
                    const fileList = stat.files.map(f => {
                        // Format state display
                        let stateDisplay = '';
                        if (f.state.type === 'Processing') {
                            stateDisplay = `PROCESSING: ${f.state.stage}`;
                        } else if (f.state.type === 'IngestComplete') {
                            stateDisplay = 'INGEST COMPLETE';
                        } else if (f.state.type === 'DuplicateHash') {
                            stateDisplay = 'DUPLICATE HASH';
                        } else if (f.state.type === 'NoAudio') {
                            stateDisplay = 'NO AUDIO';
                        } else {
                            stateDisplay = f.state.type;
                        }

                        // Format time display
                        const timeDisplay = f.total_time_seconds !== null && f.total_time_seconds !== undefined
                            ? ` (${f.total_time_seconds.toFixed(1)}s)`
                            : '';

                        return `<div class="file-status-item">#${f.file_index} ${f.file_path} - ${stateDisplay}${timeDisplay}</div>`;
                    }).join('');
                    fileSection = `<hr class="file-separator"><div class="scrollable-list file-status-list">${fileList}</div>`;
                }

                content = `Processing ${stat.completed} of ${stat.total} (${stat.started} started) ingest_max_concurrent_jobs ${stat.max_workers}${workerSection ? '<br>' + workerSection : ''}${fileSection}`;
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

function getPhaseSum(phase, status, phaseStatistics) {
    if (status === 'Completed' || status === 'CompletedWithWarnings') {
        // **[PLAN027]** For SCANNING phase, show file counts and link
        if (phase.phase === 'SCANNING') {
            // Find SCANNING statistics to get file counts
            if (phaseStatistics && phaseStatistics.length > 0) {
                const scanStat = phaseStatistics.find(s => s.phase_name === 'SCANNING');
                if (scanStat) {
                    return `Found ${scanStat.audio_files} audio files, ${scanStat.image_files} image files, ${scanStat.other_files} other files. <a href="/file-report" style="color: #4a9eff; text-decoration: underline; font-weight: bold;">View File Classification Report</a>`;
                }
            }
            return 'Completed <a href="/file-report" style="color: #4a9eff; text-decoration: underline; font-weight: bold;">View File Classification Report</a>';
        }

        return `Completed - ${phase.progress_current}/${phase.progress_total} processed`;
    } else if (status === 'InProgress') {
        // For SCANNING phase, omit progress counts
        if (phase.phase === 'SCANNING') {
            return 'In Progress';
        }
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

// **[AIA-UI-010]** Start live worker activity elapsed time updates
// Updates elapsed times every second using client-side calculation
function startWorkerLiveUpdates() {
    workerUpdateInterval = setInterval(() => {
        updateWorkerElapsedTimes();
    }, 1000); // Update every second
}

// **[AIA-UI-010]** Update worker elapsed times in DOM
// Recalculates elapsed time from phase_started_at timestamp
function updateWorkerElapsedTimes() {
    if (currentWorkerActivities.length === 0) return;

    currentWorkerActivities.forEach(worker => {
        if (!worker.phase_started_at) return;

        // Find the worker element in DOM
        const workerElement = document.querySelector(`.worker-item[data-worker-id="${worker.worker_id}"]`);
        if (!workerElement) return;

        // Find the elapsed time span within this worker element
        const elapsedSpan = workerElement.querySelector('.worker-elapsed');
        if (!elapsedSpan) return;

        // Calculate current elapsed time from phase_started_at
        const startTime = new Date(worker.phase_started_at);
        const elapsedSeconds = (Date.now() - startTime.getTime()) / 1000;
        const newElapsedDisplay = `Started ${elapsedSeconds.toFixed(1)} seconds ago.`;

        // Update only if changed (avoid unnecessary DOM updates)
        if (elapsedSpan.textContent !== newElapsedDisplay) {
            elapsedSpan.textContent = newElapsedDisplay;
        }
    });
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

// **[AIA-UI-010]** Cleanup worker interval when page unloads
window.addEventListener('beforeunload', () => {
    if (workerUpdateInterval) {
        clearInterval(workerUpdateInterval);
        workerUpdateInterval = null;
    }
});

// =============================================================================
// **[PLAN032]** Analysis Log Functions
// =============================================================================

const MAX_LOG_ENTRIES = 500;
let analysisLogEntries = [];

/**
 * Handle incoming AnalysisLog SSE event
 * @param {Object} entry - The AnalysisLogEntry object
 */
function handleAnalysisLog(entry) {
    analysisLogEntries.push(entry);

    // Trim old entries
    if (analysisLogEntries.length > MAX_LOG_ENTRIES) {
        analysisLogEntries = analysisLogEntries.slice(-MAX_LOG_ENTRIES);
        // Also trim DOM if too many entries
        const container = document.getElementById('log-entries');
        if (container && container.children.length > MAX_LOG_ENTRIES) {
            while (container.children.length > MAX_LOG_ENTRIES) {
                container.removeChild(container.firstChild);
            }
        }
    }

    appendLogEntry(entry);
}

/**
 * Append a log entry to the DOM
 * @param {Object} entry - The AnalysisLogEntry object
 */
function appendLogEntry(entry) {
    const container = document.getElementById('log-entries');
    if (!container) return;

    // Convert message type to lowercase CSS class (e.g., "AlbumMatch" -> "albummatch")
    const typeClass = entry.message_type.toLowerCase();

    // Check filter checkbox
    const checkbox = document.getElementById(`log-show-${typeClass}`);
    if (checkbox && !checkbox.checked) return;

    const div = document.createElement('div');
    div.className = `log-entry ${typeClass}`;
    div.dataset.messageType = typeClass;

    // Format timestamp HH:MM:SS
    const ts = new Date(entry.timestamp);
    const timestamp = ts.toLocaleTimeString('en-US', { hour12: false });

    // File index X/Y
    const fileIndex = `[${entry.file_index}/${entry.total_files}]`;

    div.innerHTML = `
        <span class="timestamp">${timestamp}</span>
        <span class="file-index">${fileIndex}</span>
        <span class="message">${escapeHtml(entry.message)}</span>
    `;

    // Add track details for album matches
    if (entry.details && entry.details.type === 'AlbumMatch') {
        const match = entry.details;
        const trackDetails = document.createElement('div');
        trackDetails.className = 'track-details';

        let trackHtml = match.track_errors.map(t => {
            const errClass = Math.abs(t.error_secs) > 5 ? 'track-error' : 'track-ok';
            const sign = t.error_secs >= 0 ? '+' : '';
            return `<div class="${errClass}">
                Track ${t.track_number}: ${escapeHtml(t.track_title)}
                (expected ${formatDuration(t.expected_duration_secs)},
                detected ${formatDuration(t.detected_duration_secs)},
                error: ${sign}${t.error_secs.toFixed(1)}s)
            </div>`;
        }).join('');

        trackDetails.innerHTML = trackHtml;
        div.appendChild(trackDetails);
    }

    container.appendChild(div);

    // Auto-scroll if enabled
    const autoScrollCheckbox = document.getElementById('log-autoscroll');
    if (autoScrollCheckbox && autoScrollCheckbox.checked) {
        container.scrollTop = container.scrollHeight;
    }
}

/**
 * Format duration in seconds to MM:SS
 * @param {number} secs - Duration in seconds
 * @returns {string} Formatted duration
 */
function formatDuration(secs) {
    const mins = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${mins}:${s.toString().padStart(2, '0')}`;
}

/**
 * Escape HTML special characters
 * @param {string} text - Text to escape
 * @returns {string} Escaped text
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Clear the analysis log
 */
function clearAnalysisLog() {
    analysisLogEntries = [];
    const container = document.getElementById('log-entries');
    if (container) container.innerHTML = '';
}

/**
 * Refresh log display based on current filter settings
 * Called when filter checkboxes change
 */
function refreshLogDisplay() {
    const container = document.getElementById('log-entries');
    if (!container) return;

    // Clear and re-render all entries with current filters
    container.innerHTML = '';
    analysisLogEntries.forEach(entry => appendLogEntry(entry));
}

// Attach filter checkbox event listeners
document.addEventListener('DOMContentLoaded', () => {
    const filterCheckboxes = [
        'log-show-info',
        'log-show-success',
        'log-show-warning',
        'log-show-error',
        'log-show-albummatch',
        'log-show-flavorlookup'
    ];

    filterCheckboxes.forEach(id => {
        const checkbox = document.getElementById(id);
        if (checkbox) {
            checkbox.addEventListener('change', refreshLogDisplay);
        }
    });
});
