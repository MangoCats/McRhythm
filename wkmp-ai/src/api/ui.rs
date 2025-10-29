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
        #status {
            background: #f5f5f5;
            padding: 20px;
            border-radius: 4px;
            margin: 20px 0;
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
    </style>
</head>
<body>
    <h1>Import Progress</h1>

    <div id="status">
        <p><strong>Status:</strong> <span id="state">Ready</span></p>
        <p><strong>Progress:</strong> <span id="progress-text">0 / 0</span></p>
        <div class="progress-bar">
            <div class="progress-fill" id="progress-bar" style="width: 0%"></div>
        </div>
        <p><strong>Current Operation:</strong> <span id="operation">-</span></p>
    </div>

    <p><a href="/">← Back to Home</a></p>

    <script>
        // SSE client will be implemented in Phase 9
        console.log('Import progress page loaded');
        // TODO: Connect to /import/events SSE endpoint
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
