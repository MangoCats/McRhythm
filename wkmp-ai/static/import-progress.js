// Import Progress Page JavaScript
// REQ-AIA-UI-001 through REQ-AIA-UI-006: Enhanced multi-level progress display

let eventSource = null;
let currentSessionId = null;
let lastUpdateTime = 0;
const UPDATE_THROTTLE_MS = 100; // REQ-AIA-UI-NF-001: Max 10 updates/sec

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
    startBtn.textContent = 'Starting...';
    errorDiv.style.display = 'none';

    try {
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

console.log('Enhanced import progress page loaded (PLAN011)');

// Connect to general SSE for connection status monitoring on page load
// This is separate from the import-specific SSE that connects when "Start Import" is clicked
// Note: WkmpSSEConnection class is loaded from /static/wkmp-sse.js in the HTML
const generalSSE = new WkmpSSEConnection('/events', 'connection-status');
generalSSE.connect();
