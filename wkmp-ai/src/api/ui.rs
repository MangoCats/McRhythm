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
    Router::new()
        .route("/", get(root_page))
        .route("/import-progress", get(import_progress_page))
        .route("/segment-editor", get(segment_editor_page))
        .route("/import-complete", get(import_complete_page))
}

/// Root page - Audio Import Home
/// **[AIA-UI-010]** HTML entry point
async fn root_page() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>wkmp-ai - Audio Import</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 40px auto;
            padding: 20px;
            line-height: 1.6;
        }
        h1 {
            color: #333;
            border-bottom: 2px solid #0066cc;
            padding-bottom: 10px;
        }
        .button {
            display: inline-block;
            padding: 10px 20px;
            background: #0066cc;
            color: white;
            text-decoration: none;
            border-radius: 4px;
            margin: 10px 5px;
        }
        .button:hover {
            background: #0052a3;
        }
    </style>
</head>
<body>
    <h1>wkmp-ai - Audio Import</h1>
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
        <a href="/import-progress" class="button">Start Import</a>
        <a href="/segment-editor" class="button">Segment Editor</a>
    </p>

    <p><small>Module: wkmp-ai v0.1.0 | Port 5723</small></p>
</body>
</html>
        "#,
    )
}

/// Import Progress Page - Live progress updates via SSE
/// **[AIA-SSE-010]** Real-time progress updates
async fn import_progress_page() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Import Progress - wkmp-ai</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 900px;
            margin: 40px auto;
            padding: 20px;
        }
        h1 {
            color: #333;
            border-bottom: 2px solid #0066cc;
            padding-bottom: 10px;
        }
        #setup {
            background: #f5f5f5;
            padding: 20px;
            border-radius: 4px;
            margin: 20px 0;
        }
        #status {
            background: #f5f5f5;
            padding: 20px;
            border-radius: 4px;
            margin: 20px 0;
            display: none;
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
        .progress-bar {
            width: 100%;
            height: 30px;
            background: #e0e0e0;
            border-radius: 4px;
            overflow: hidden;
            margin: 10px 0;
        }
        .progress-fill {
            height: 100%;
            background: #0066cc;
            transition: width 0.3s ease;
        }
        .error {
            background: #ffebee;
            color: #c62828;
            padding: 10px;
            border-radius: 4px;
            margin: 10px 0;
        }
    </style>
</head>
<body>
    <h1>Import Progress</h1>

    <div id="setup">
        <div class="form-group">
            <label for="root-folder">Music Root Folder:</label>
            <input type="text" id="root-folder" placeholder="/home/sw/Music" value="/home/sw/Music">
        </div>
        <button class="button" id="start-btn" onclick="startImport()">Start Import</button>
        <div id="error" class="error" style="display: none;"></div>
    </div>

    <div id="status">
        <p><strong>Session ID:</strong> <span id="session-id">-</span></p>
        <p><strong>Status:</strong> <span id="state">-</span></p>
        <p><strong>Progress:</strong> <span id="progress-text">0 / 0</span></p>
        <div class="progress-bar">
            <div class="progress-fill" id="progress-bar" style="width: 0%"></div>
        </div>
        <p><strong>Current Operation:</strong> <span id="operation">-</span></p>
    </div>

    <p><a href="/">← Back to Home</a></p>

    <script>
        let eventSource = null;
        let currentSessionId = null;

        // Start import workflow
        async function startImport() {
            const rootFolder = document.getElementById('root-folder').value.trim();
            const startBtn = document.getElementById('start-btn');
            const errorDiv = document.getElementById('error');

            if (!rootFolder) {
                showError('Please enter a root folder path');
                return;
            }

            // Disable button
            startBtn.disabled = true;
            startBtn.textContent = 'Starting...';
            errorDiv.style.display = 'none';

            try {
                // Call POST /import/start
                const response = await fetch('/import/start', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ root_folder: rootFolder })
                });

                if (!response.ok) {
                    const error = await response.json();
                    // Error format: { error: { code: "...", message: "..." } }
                    const errorMessage = error?.error?.message || error?.message || 'Failed to start import';
                    throw new Error(errorMessage);
                }

                const data = await response.json();
                currentSessionId = data.session_id;

                // Show status panel, hide setup
                document.getElementById('setup').style.display = 'none';
                document.getElementById('status').style.display = 'block';
                document.getElementById('session-id').textContent = currentSessionId;
                document.getElementById('state').textContent = data.state;

                // Connect to SSE stream
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

            eventSource.addEventListener('ImportSessionStarted', (e) => {
                const event = JSON.parse(e.data);
                console.log('ImportSessionStarted:', event);
                document.getElementById('state').textContent = 'Started';
            });

            eventSource.addEventListener('ImportProgressUpdate', (e) => {
                const event = JSON.parse(e.data);
                console.log('ImportProgressUpdate:', event);

                // Event fields are at top level, not nested under 'progress'
                const percent = event.total > 0
                    ? Math.round((event.current / event.total) * 100)
                    : 0;

                document.getElementById('state').textContent = event.state;
                document.getElementById('progress-text').textContent =
                    `${event.current} / ${event.total}`;
                document.getElementById('progress-bar').style.width = `${percent}%`;
                document.getElementById('operation').textContent = event.current_operation || '';
            });

            eventSource.addEventListener('ImportSessionCompleted', (e) => {
                const event = JSON.parse(e.data);
                console.log('ImportSessionCompleted:', event);

                document.getElementById('state').textContent = 'Completed ✓';
                document.getElementById('operation').textContent = 'Import finished successfully';
                eventSource.close();

                // Redirect to completion page after 2 seconds
                setTimeout(() => {
                    window.location.href = '/import-complete';
                }, 2000);
            });

            eventSource.addEventListener('ImportSessionFailed', (e) => {
                const event = JSON.parse(e.data);
                console.log('ImportSessionFailed:', event);

                document.getElementById('state').textContent = 'Failed';
                document.getElementById('operation').textContent = 'Import failed';
                showError(event.error || 'Import failed');
                eventSource.close();
            });

            eventSource.addEventListener('ImportSessionCancelled', (e) => {
                const event = JSON.parse(e.data);
                console.log('ImportSessionCancelled:', event);

                document.getElementById('state').textContent = 'Cancelled';
                document.getElementById('operation').textContent = 'Import cancelled';
                eventSource.close();
            });

            eventSource.onerror = (error) => {
                console.error('SSE error:', error);
                // Don't show error immediately - SSE may reconnect
            };
        }

        // Show error message
        function showError(message) {
            const errorDiv = document.getElementById('error');
            errorDiv.textContent = message;
            errorDiv.style.display = 'block';
        }

        console.log('Import progress page loaded');
    </script>
</body>
</html>
        "#,
    )
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
