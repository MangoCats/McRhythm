//! UI Routes - HTML pages for wkmp-ai web interface
//!
//! **[AIA-UI-010]** Web UI with HTML/CSS/JS (vanilla ES6+, no frameworks)
//! **[AIA-UI-030]** Return navigation to wkmp-ui on completion

use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

use crate::AppState;

/// Build UI routes
pub fn ui_routes() -> Router<AppState> {
    use tower_http::services::ServeDir;

    Router::new()
        .route("/", get(root_page))
        .route("/import-progress", get(import_progress_page))
        .route("/segment-editor", get(segment_editor_page))
        .route("/import-complete", get(import_complete_page))
        .route("/settings", get(settings_page))
        .nest_service("/static", ServeDir::new("wkmp-ai/static"))
}

/// Root page - Audio Import Home
/// **[AIA-UI-010]** HTML entry point
async fn root_page() -> impl IntoResponse {
    let build_timestamp = env!("BUILD_TIMESTAMP");

    let html = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>wkmp-ai - Audio Import</title>
    <style>
        body {{
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 40px auto;
            padding: 20px;
            line-height: 1.6;
        }}
        .build-info {{
            position: fixed;
            top: 10px;
            left: 10px;
            font-size: 11px;
            color: #666;
            background: #f5f5f5;
            padding: 4px 8px;
            border-radius: 3px;
            font-family: monospace;
        }}
        h1 {{
            color: #333;
            border-bottom: 2px solid #0066cc;
            padding-bottom: 10px;
        }}
        .button {{
            display: inline-block;
            padding: 10px 20px;
            background: #0066cc;
            color: white;
            text-decoration: none;
            border-radius: 4px;
            margin: 10px 5px;
        }}
        .button:hover {{
            background: #0052a3;
        }}
    </style>
</head>
<body>
    <div class="build-info">Built: {}</div>
    <h1>wkmp-ai - Audio Import</h1>"#,
        build_timestamp
    );

    Html(format!(
        "{}
    <p>Import your music collection into WKMP with automatic MusicBrainz identification and passage boundary detection.</p>

    <h2>Features</h2>
    <ul>
        <li>Automatic audio file discovery</li>
        <li>MusicBrainz & AcoustID identification</li>
        <li>Silence-based passage boundary detection</li>
        <li>Amplitude analysis for crossfade timing</li>
        <li>Musical flavor extraction (Essentia)</li>
    </ul>

    <h2>Quick Start</h2>
    <p>
        <a href=\"/import-progress\" class=\"button\">Start Import</a>
        <a href=\"/segment-editor\" class=\"button\">Segment Editor</a>
        <a href=\"http://localhost:5725/\" target=\"_blank\" class=\"button\">Database Review</a>
    </p>

    <p><small>Module: wkmp-ai v0.1.0 | Port 5723</small></p>
</body>
</html>",
        html
    ))
}

/// Import Progress Page - Live progress updates via SSE
/// **[AIA-SSE-010]** Real-time progress updates
/// **[REQ-AIA-UI-001 through REQ-AIA-UI-006]** Enhanced multi-level progress display
async fn import_progress_page() -> impl IntoResponse {
    // Get platform-appropriate default root folder path [REQ-NF-033]
    let default_root = wkmp_common::config::get_default_root_folder();
    let default_root_str = default_root.to_string_lossy();

    let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Import Progress - wkmp-ai</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 1000px;
            margin: 20px auto;
            padding: 20px;
            background: #f5f5f5;
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #0066cc;
            padding-bottom: 10px;
        }
        #setup {
            background: white;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .form-group {
            margin: 15px 0;
        }
        .form-group label {
            display: block;
            font-weight: bold;
            margin-bottom: 5px;
        }
        .form-group input {
            width: 100%;
            padding: 8px;
            border: 1px solid #ccc;
            border-radius: 4px;
            font-size: 14px;
            box-sizing: border-box;
        }
        .button {
            display: inline-block;
            padding: 10px 20px;
            background: #0066cc;
            color: white;
            text-decoration: none;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 16px;
        }
        .button:hover {
            background: #0052a3;
        }
        .button:disabled {
            background: #999;
            cursor: not-allowed;
        }
        .error {
            background: #ffebee;
            color: #c62828;
            padding: 10px;
            border-radius: 4px;
            margin: 10px 0;
        }

        /* REQ-AIA-UI-001: Workflow Checklist */
        .workflow-checklist {
            background: white;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            display: none;
        }
        .workflow-checklist h2 {
            margin-top: 0;
            color: #333;
        }
        .phase-item {
            padding: 12px;
            margin: 8px 0;
            border-left: 4px solid #ddd;
            background: #f9f9f9;
            display: flex;
            align-items: center;
        }
        .phase-item.pending { border-color: #ccc; }
        .phase-item.in-progress { border-color: #0066cc; background: #e3f2fd; }
        .phase-item.completed { border-color: #4caf50; background: #e8f5e9; }
        .phase-item.failed { border-color: #f44336; background: #ffebee; }
        .phase-icon {
            font-size: 24px;
            margin-right: 12px;
            width: 30px;
            text-align: center;
        }
        .phase-content {
            flex: 1;
        }
        .phase-name {
            font-weight: bold;
            font-size: 16px;
        }
        .phase-description {
            color: #666;
            font-size: 13px;
            font-style: italic;
            margin-top: 2px;
        }
        .phase-summary {
            color: #666;
            font-size: 14px;
            margin-top: 4px;
        }

        /* REQ-AIA-UI-002: Active Phase Progress */
        .active-progress {
            background: white;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            display: none;
        }
        .progress-header {
            display: flex;
            justify-content: space-between;
            margin-bottom: 10px;
        }
        .progress-bar {
            width: 100%;
            height: 30px;
            background: #e0e0e0;
            border-radius: 15px;
            overflow: hidden;
            margin: 10px 0;
        }
        .progress-fill {
            height: 100%;
            background: linear-gradient(90deg, #0066cc, #0052a3);
            transition: width 0.3s ease;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-weight: bold;
        }

        /* REQ-AIA-UI-003: Sub-Task Status */
        .subtask-status {
            background: white;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            display: none;
        }
        .subtask-item {
            padding: 10px;
            margin: 8px 0;
            border-left: 4px solid #ddd;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .subtask-item.green { border-color: #4caf50; background: #e8f5e9; }
        .subtask-item.yellow { border-color: #ff9800; background: #fff3e0; }
        .subtask-item.red { border-color: #f44336; background: #ffebee; }
        .subtask-name {
            font-weight: bold;
        }
        .subtask-stats {
            color: #666;
            font-size: 14px;
        }

        /* REQ-AIA-UI-004: Current File Display */
        .current-file {
            background: white;
            padding: 15px;
            border-radius: 8px;
            margin: 20px 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            display: none;
        }
        .current-file-label {
            font-weight: bold;
            color: #666;
            font-size: 14px;
        }
        .current-file-path {
            font-family: monospace;
            font-size: 13px;
            color: #333;
            margin-top: 5px;
            word-break: break-all;
        }

        /* REQ-AIA-UI-005: Time Estimates */
        .time-estimates {
            display: flex;
            gap: 20px;
            background: white;
            padding: 15px;
            border-radius: 8px;
            margin: 20px 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            display: none;
        }
        .time-item {
            flex: 1;
        }
        .time-label {
            font-weight: bold;
            color: #666;
            font-size: 14px;
        }
        .time-value {
            font-size: 24px;
            color: #0066cc;
            font-weight: bold;
            margin-top: 5px;
        }

        /* REQ-AIA-UI-NF-002: Mobile responsive */
        @media (max-width: 768px) {
            body {
                margin: 10px;
                padding: 10px;
            }
            .time-estimates {
                flex-direction: column;
                gap: 10px;
            }
        }
    </style>
</head>
<body>
    <h1>Import Progress</h1>

    <div id="setup">
        <div class="form-group">
            <label for="root-folder">Music Root Folder:</label>
            <input type="text" id="root-folder" placeholder="DEFAULT_ROOT_PLACEHOLDER" value="DEFAULT_ROOT_PLACEHOLDER">
        </div>
        <button class="button" id="start-btn" onclick="startImport()">Start Import</button>
        <div id="error" class="error" style="display: none;"></div>
    </div>

    <!-- REQ-AIA-UI-001: Workflow Checklist -->
    <div class="workflow-checklist" id="workflow-checklist">
        <h2>Workflow Progress</h2>
        <div id="phases-container"></div>
    </div>

    <!-- REQ-AIA-UI-002: Active Phase Progress -->
    <div class="active-progress" id="active-progress">
        <h2 id="current-phase-name">Current Phase</h2>
        <div class="progress-header">
            <span id="progress-text">0 / 0 files</span>
            <span id="progress-percent">0%</span>
        </div>
        <div class="progress-bar">
            <div class="progress-fill" id="progress-bar" style="width: 0%">0%</div>
        </div>
    </div>

    <!-- REQ-AIA-UI-003: Sub-Task Status -->
    <div class="subtask-status" id="subtask-status">
        <h2>Sub-Task Status</h2>
        <div id="subtasks-container"></div>
    </div>

    <!-- REQ-AIA-UI-004: Current File Display -->
    <div class="current-file" id="current-file">
        <div class="current-file-label">Currently Processing:</div>
        <div class="current-file-path" id="current-file-path">-</div>
    </div>

    <!-- REQ-AIA-UI-005: Time Estimates -->
    <div class="time-estimates" id="time-estimates">
        <div class="time-item">
            <div class="time-label">Elapsed Time</div>
            <div class="time-value" id="elapsed-time">0s</div>
        </div>
        <div class="time-item">
            <div class="time-label">Estimated Remaining</div>
            <div class="time-value" id="remaining-time">Estimating...</div>
        </div>
    </div>

    <p><a href="/">← Back to Home</a></p>

    <script>
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

        // Connect to SSE event stream
        function connectSSE() {
            console.log('Connecting to SSE at /import/events');
            eventSource = new EventSource('/import/events');

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
    </script>
</body>
</html>
        "#;

    // Replace placeholder with actual default root folder path
    let html = html.replace("DEFAULT_ROOT_PLACEHOLDER", &default_root_str);

    Html(html)
}

/// Segment Editor Page - Waveform visualization with draggable markers
/// **[AIA-UI-010]** Waveform editor for passage boundaries
/// **Decision 2:** Client-side Canvas API for waveform rendering
async fn segment_editor_page() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Segment Editor - wkmp-ai</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 1200px;
            margin: 40px auto;
            padding: 20px;
        }
        h1 {
            color: #333;
            border-bottom: 2px solid #0066cc;
            padding-bottom: 10px;
        }
        #waveform-container {
            width: 100%;
            height: 200px;
            background: #f5f5f5;
            border: 1px solid #ccc;
            border-radius: 4px;
            margin: 20px 0;
            position: relative;
        }
        canvas {
            width: 100%;
            height: 100%;
        }
    </style>
</head>
<body>
    <h1>Segment Editor</h1>

    <p>Adjust passage boundaries by dragging markers on the waveform.</p>

    <div id="waveform-container">
        <canvas id="waveform" width="1200" height="200"></canvas>
    </div>

    <p><strong>Instructions:</strong> Click and drag markers to adjust passage boundaries. Changes are saved automatically.</p>

    <p><a href="/">← Back to Home</a></p>

    <script>
        // Canvas API waveform rendering (Decision 2)
        const canvas = document.getElementById('waveform');
        const ctx = canvas.getContext('2d');

        // Draw placeholder waveform
        ctx.fillStyle = '#0066cc';
        ctx.fillRect(0, 100, canvas.width, 2);
        ctx.font = '14px system-ui';
        ctx.fillText('Waveform visualization will be implemented in Phase 13', 20, 50);

        console.log('Segment editor loaded (Canvas API ready)');
        // TODO: Implement waveform rendering and boundary markers
    </script>
</body>
</html>
        "#,
    )
}

/// Import Complete Page - Summary with return link to wkmp-ui
/// **[AIA-UI-030]** Return navigation to wkmp-ui
async fn import_complete_page() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Import Complete - wkmp-ai</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 40px auto;
            padding: 20px;
            text-align: center;
        }
        h1 {
            color: #0066cc;
        }
        .button {
            display: inline-block;
            padding: 15px 30px;
            background: #0066cc;
            color: white;
            text-decoration: none;
            border-radius: 4px;
            margin: 20px;
            font-size: 16px;
        }
        .button:hover {
            background: #0052a3;
        }
        .success {
            color: #008800;
            font-size: 48px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="success">✓</div>
    <h1>Import Complete!</h1>
    <p>Your music collection has been successfully imported.</p>

    <div>
        <a href="http://localhost:5720" class="button">Return to wkmp-ui</a>
        <a href="/" class="button">Start Another Import</a>
    </div>

    <p><small>wkmp-ai v0.1.0</small></p>
</body>
</html>
        "#,
    )
}

/// Settings Page - AcoustID API key configuration
/// **[APIK-UI-040]** Settings page with API key input
async fn settings_page() -> impl IntoResponse {
    // Read static HTML file
    match tokio::fs::read_to_string("wkmp-ai/static/settings.html").await {
        Ok(content) => Html(content).into_response(),
        Err(e) => {
            tracing::error!("Failed to read settings.html: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Settings page not found",
            )
                .into_response()
        }
    }
}
