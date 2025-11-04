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
    use tower_http::set_header::SetResponseHeaderLayer;
    use axum::http::header;
    use tower::ServiceBuilder;

    Router::new()
        .route("/", get(root_page))
        .route("/import-progress", get(import_progress_page))
        .route("/segment-editor", get(segment_editor_page))
        .route("/import-complete", get(import_complete_page))
        .route("/settings", get(settings_page))
        .nest_service(
            "/static",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    "no-cache, no-store, must-revalidate".parse::<axum::http::HeaderValue>().unwrap(),
                ))
                .service(ServeDir::new("wkmp-ai/static"))
        )
}

/// Root page - Audio Import Home
/// **[AIA-UI-010]** HTML entry point
async fn root_page() -> impl IntoResponse {
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = env!("GIT_HASH");
    let build_profile = env!("BUILD_PROFILE");

    let html = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WKMP Audio Import</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background-color: #1a1a1a;
            color: #e0e0e0;
            line-height: 1.6;
        }}
        .container {{
            padding: 20px;
        }}
        header {{
            background-color: #2a2a2a;
            border-bottom: 1px solid #3a3a3a;
            padding: 20px;
            margin-bottom: 30px;
        }}
        .header-content {{
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .header-left {{
            flex: 1;
        }}
        .header-right {{
            text-align: right;
            font-size: 16px;
            color: #888;
            font-family: 'Courier New', monospace;
            line-height: 1.2;
        }}
        .build-info-line {{
            margin-bottom: 1px;
        }}
        h1 {{
            font-size: 26px;
            margin-bottom: 5px;
            color: #4a9eff;
        }}
        .subtitle {{
            color: #888;
            font-size: 16px;
        }}
        .content {{
            padding: 0 20px;
        }}
        h2 {{
            color: #4a9eff;
            margin-top: 20px;
            margin-bottom: 10px;
        }}
        ul {{
            margin-left: 20px;
            margin-bottom: 20px;
        }}
        .button {{
            display: inline-block;
            padding: 10px 20px;
            background: #4a9eff;
            color: white;
            text-decoration: none;
            border-radius: 4px;
            margin: 10px 5px;
            font-weight: 600;
        }}
        .button:hover {{
            background: #3a8eef;
        }}
    </style>
</head>
<body>
    <header>
        <div class="header-content">
            <div class="header-left">
                <h1>WKMP Audio Import</h1>
                <p class="subtitle">Music collection import and identification</p>
            </div>
            <div class="header-right">
                <div class="build-info-line">wkmp-ai v{}</div>
                <div class="build-info-line">{} ({})</div>
                <div class="build-info-line">{}</div>
            </div>
        </div>
    </header>
    <div class="content">"#,
        version, &git_hash[..8], build_profile, build_timestamp
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
        <a href=\"/settings\" class=\"button\">Settings</a>
        <a href=\"http://localhost:5725/\" target=\"_blank\" class=\"button\">Database Review</a>
    </p>
    </div>
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

    let version = env!("CARGO_PKG_VERSION");
    let git_hash = env!("GIT_HASH");
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let build_profile = env!("BUILD_PROFILE");

    let html = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WKMP Audio Import - Progress</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #1a1a1a;
            color: #e0e0e0;
        }}
        header {{
            background-color: #2a2a2a;
            border-bottom: 1px solid #3a3a3a;
            padding: 20px;
            margin-bottom: 30px;
        }}
        .header-content {{
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .header-left {{
            flex: 1;
        }}
        .header-right {{
            text-align: right;
            font-size: 16px;
            color: #888;
            font-family: 'Courier New', monospace;
            line-height: 1.2;
        }}
        .build-info-line {{
            margin-bottom: 1px;
        }}
        h1 {{
            font-size: 26px;
            margin-bottom: 5px;
            color: #4a9eff;
        }}
        .subtitle {{
            color: #888;
            font-size: 16px;
        }}
        .content {{
            padding: 0 20px;
        }}
        h2 {{
            color: #4a9eff;
        }}
        #setup {{
            background: #2a2a2a;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            border: 1px solid #3a3a3a;
            box-shadow: 0 2px 4px rgba(0,0,0,0.3);
        }}
        .form-group {{
            margin: 15px 0;
        }}
        .form-group label {{
            display: block;
            font-weight: bold;
            margin-bottom: 5px;
            color: #e0e0e0;
        }}
        .form-group input {{
            width: 100%;
            padding: 8px;
            border: 1px solid #3a3a3a;
            border-radius: 4px;
            font-size: 14px;
            box-sizing: border-box;
            background: #333;
            color: #e0e0e0;
        }}
        .button {{
            display: inline-block;
            padding: 10px 20px;
            background: #4a9eff;
            color: white;
            text-decoration: none;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 16px;
            font-weight: 600;
        }}
        .button:hover {{
            background: #3a8eef;
        }}
        .button:disabled {{
            background: #666;
            cursor: not-allowed;
        }}
        .error {{
            background: rgba(220, 38, 38, 0.2);
            color: #dc2626;
            padding: 10px;
            border-radius: 4px;
            border: 1px solid #dc2626;
            margin: 10px 0;
        }}

        /* REQ-AIA-UI-001: Workflow Checklist */
        .workflow-checklist {{
            background: #2a2a2a;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            border: 1px solid #3a3a3a;
            box-shadow: 0 2px 4px rgba(0,0,0,0.3);
            display: none;
        }}
        .workflow-checklist h2 {{
            margin-top: 0;
            color: #4a9eff;
        }}
        .phase-item {{
            padding: 12px;
            margin: 8px 0;
            border-left: 4px solid #3a3a3a;
            background: #333;
            display: flex;
            align-items: center;
        }}
        .phase-item.pending {{ border-color: #666; }}
        .phase-item.in-progress {{ border-color: #4a9eff; background: rgba(74, 158, 255, 0.2); }}
        .phase-item.completed {{ border-color: #4ade80; background: rgba(74, 222, 128, 0.2); }}
        .phase-item.failed {{ border-color: #dc2626; background: rgba(220, 38, 38, 0.2); }}
        .phase-icon {{
            font-size: 24px;
            margin-right: 12px;
            width: 30px;
            text-align: center;
        }}
        .phase-content {{
            flex: 1;
        }}
        .phase-name {{
            font-weight: bold;
            font-size: 16px;
        }}
        .phase-description {{
            color: #888;
            font-size: 13px;
            font-style: italic;
            margin-top: 2px;
        }}
        .phase-summary {{
            color: #888;
            font-size: 14px;
            margin-top: 4px;
        }}

        /* REQ-AIA-UI-002: Active Phase Progress */
        .active-progress {{
            background: #2a2a2a;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            border: 1px solid #3a3a3a;
            box-shadow: 0 2px 4px rgba(0,0,0,0.3);
            display: none;
        }}
        .progress-header {{
            display: flex;
            justify-content: space-between;
            margin-bottom: 10px;
        }}
        .progress-bar {{
            width: 100%;
            height: 30px;
            background: #333;
            border-radius: 15px;
            overflow: hidden;
            margin: 10px 0;
        }}
        .progress-fill {{
            height: 100%;
            background: linear-gradient(90deg, #4a9eff, #3a8eef);
            transition: width 0.3s ease;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-weight: bold;
        }}

        /* REQ-AIA-UI-003: Sub-Task Status */
        .subtask-status {{
            background: #2a2a2a;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            border: 1px solid #3a3a3a;
            box-shadow: 0 2px 4px rgba(0,0,0,0.3);
            display: none;
        }}
        .subtask-item {{
            padding: 10px;
            margin: 8px 0;
            border-left: 4px solid #3a3a3a;
            background: #333;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .subtask-item.green {{ border-color: #4ade80; background: rgba(74, 222, 128, 0.2); }}
        .subtask-item.yellow {{ border-color: #ff9800; background: rgba(255, 152, 0, 0.2); }}
        .subtask-item.red {{ border-color: #dc2626; background: rgba(220, 38, 38, 0.2); }}
        .subtask-name {{
            font-weight: bold;
        }}
        .subtask-stats {{
            color: #888;
            font-size: 14px;
        }}

        /* REQ-AIA-UI-004: Current File Display */
        .current-file {{
            background: #2a2a2a;
            padding: 15px;
            border-radius: 8px;
            margin: 20px 0;
            border: 1px solid #3a3a3a;
            box-shadow: 0 2px 4px rgba(0,0,0,0.3);
            display: none;
        }}
        .current-file-label {{
            font-weight: bold;
            color: #888;
            font-size: 14px;
        }}
        .current-file-path {{
            font-family: monospace;
            font-size: 13px;
            color: #e0e0e0;
            margin-top: 5px;
            word-break: break-all;
        }}

        /* REQ-AIA-UI-005: Time Estimates */
        .time-estimates {{
            display: flex;
            gap: 20px;
            background: #2a2a2a;
            padding: 15px;
            border-radius: 8px;
            margin: 20px 0;
            border: 1px solid #3a3a3a;
            box-shadow: 0 2px 4px rgba(0,0,0,0.3);
            display: none;
        }}
        .time-item {{
            flex: 1;
        }}
        .time-label {{
            font-weight: bold;
            color: #888;
            font-size: 14px;
        }}
        .time-value {{
            font-size: 24px;
            color: #4a9eff;
            font-weight: bold;
            margin-top: 5px;
        }}

        /* REQ-AIA-UI-NF-002: Mobile responsive */
        @media (max-width: 768px) {{
            body {{
                margin: 10px;
                padding: 10px;
            }}
            .time-estimates {{
                flex-direction: column;
                gap: 10px;
            }}
        }}
    </style>
</head>
<body>
    <header>
        <div class="header-content">
            <div class="header-left">
                <h1>WKMP Audio Import</h1>
                <p class="subtitle">Live import progress and status</p>
            </div>
            <div class="header-right">
                <div class="build-info-line">v{0}</div>
                <div class="build-info-line">{1} ({2})</div>
                <div class="build-info-line">{3}</div>
            </div>
        </div>
    </header>
    <div class="content">

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

    <script src="/static/import-progress.js"></script>
    <script type="module">
        // Initialize default root folder placeholder
        const rootFolderInput = document.getElementById('root-folder');
        if (rootFolderInput) {{
            rootFolderInput.placeholder = '{4}';
            rootFolderInput.value = '{4}';
        }}
    </script>
    </div>
</body>
</html>
        "#, version, &git_hash[..8], build_profile, build_timestamp, &default_root_str);

    Html(html)
}

/// Segment Editor Page - Waveform visualization with draggable markers
/// **[AIA-UI-010]** Waveform editor for passage boundaries
/// **Decision 2:** Client-side Canvas API for waveform rendering
async fn segment_editor_page() -> impl IntoResponse {
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = env!("GIT_HASH");
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let build_profile = env!("BUILD_PROFILE");

    Html(format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WKMP Audio Import - Segment Editor</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #1a1a1a;
            color: #e0e0e0;
        }}
        header {{
            background-color: #2a2a2a;
            border-bottom: 1px solid #3a3a3a;
            padding: 20px;
            margin-bottom: 30px;
        }}
        .header-content {{
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .header-left {{
            flex: 1;
        }}
        .header-right {{
            text-align: right;
            font-size: 16px;
            color: #888;
            font-family: 'Courier New', monospace;
            line-height: 1.2;
        }}
        .build-info-line {{
            margin-bottom: 1px;
        }}
        h1 {{
            font-size: 26px;
            margin-bottom: 5px;
            color: #4a9eff;
        }}
        .subtitle {{
            color: #888;
            font-size: 16px;
        }}
        .content {{
            padding: 0 20px;
        }}
        h2 {{
            color: #4a9eff;
        }}
        a {{
            color: #4a9eff;
        }}
        #waveform-container {{
            width: 100%;
            height: 200px;
            background: #2a2a2a;
            border: 1px solid #3a3a3a;
            border-radius: 4px;
            margin: 20px 0;
            position: relative;
        }}
        canvas {{
            width: 100%;
            height: 100%;
        }}
    </style>
</head>
<body>
    <header>
        <div class="header-content">
            <div class="header-left">
                <h1>WKMP Audio Import</h1>
                <p class="subtitle">Passage boundary segment editor</p>
            </div>
            <div class="header-right">
                <div class="build-info-line">v{0}</div>
                <div class="build-info-line">{1} ({2})</div>
                <div class="build-info-line">{3}</div>
            </div>
        </div>
    </header>
    <div class="content">

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
        ctx.fillStyle = '#4a9eff';
        ctx.fillRect(0, 100, canvas.width, 2);
        ctx.font = '14px system-ui';
        ctx.fillStyle = '#e0e0e0';
        ctx.fillText('Waveform visualization will be implemented in Phase 13', 20, 50);

        console.log('Segment editor loaded (Canvas API ready)');
        // TODO: Implement waveform rendering and boundary markers
    </script>
    </div>
</body>
</html>
        "#, version, &git_hash[..8], build_profile, build_timestamp
    ))
}

/// Import Complete Page - Summary with return link to wkmp-ui
/// **[AIA-UI-030]** Return navigation to wkmp-ui
async fn import_complete_page() -> impl IntoResponse {
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = env!("GIT_HASH");
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let build_profile = env!("BUILD_PROFILE");

    Html(format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WKMP Audio Import - Complete</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #1a1a1a;
            color: #e0e0e0;
        }}
        header {{
            background-color: #2a2a2a;
            border-bottom: 1px solid #3a3a3a;
            padding: 20px;
            margin-bottom: 30px;
        }}
        .header-content {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            max-width: 1200px;
            margin: 0 auto;
        }}
        .header-left {{
            flex: 1;
        }}
        .header-right {{
            text-align: right;
            font-size: 16px;
            color: #888;
            font-family: 'Courier New', monospace;
            line-height: 1.2;
        }}
        .build-info-line {{
            margin-bottom: 1px;
        }}
        h1 {{
            font-size: 26px;
            margin-bottom: 5px;
            color: #4a9eff;
        }}
        .subtitle {{
            color: #888;
            font-size: 16px;
        }}
        .content {{
            padding: 40px 20px;
            text-align: center;
        }}
        h2 {{
            color: #4a9eff;
            margin-bottom: 20px;
        }}
        .button {{
            display: inline-block;
            padding: 15px 30px;
            background: #4a9eff;
            color: white;
            text-decoration: none;
            border-radius: 4px;
            margin: 10px;
            font-size: 16px;
            font-weight: 600;
        }}
        .button:hover {{
            background: #3a8eef;
        }}
        .success {{
            color: #4ade80;
            font-size: 48px;
            margin: 20px 0;
        }}
    </style>
</head>
<body>
    <header>
        <div class="header-content">
            <div class="header-left">
                <h1>WKMP Audio Import</h1>
                <p class="subtitle">Import workflow complete</p>
            </div>
            <div class="header-right">
                <div class="build-info-line">v{0}</div>
                <div class="build-info-line">{1} ({2})</div>
                <div class="build-info-line">{3}</div>
            </div>
        </div>
    </header>
    <div class="content">
    <div class="success">✓</div>
    <h2>Import Complete!</h2>
    <p>Your music collection has been successfully imported.</p>

    <div>
        <a href="http://localhost:5720" class="button">Return to wkmp-ui</a>
        <a href="/" class="button">Start Another Import</a>
    </div>
    </div>
</body>
</html>
        "#, version, &git_hash[..8], build_profile, build_timestamp
    ))
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
