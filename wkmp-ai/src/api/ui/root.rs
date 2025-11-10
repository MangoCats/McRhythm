//! Root page handler - Import wizard landing page

use axum::response::{Html, IntoResponse};

/// GET /
///
/// Import wizard landing page with folder selection
pub async fn root_page() -> impl IntoResponse {
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
    <link rel="stylesheet" href=\"/static/wkmp-ui.css\">
    <style>
        /* Module-specific styles - shared styles in wkmp-ui.css */
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
            display: flex;
            align-items: center;
            gap: 10px;
        }}
        .subtitle {{
            color: #888;
            font-size: 16px;
        }}
        .connection-status {{
            display: inline-block;
            padding: 3px 8px;
            border-radius: 10px;
            font-size: 12px;
            font-weight: 600;
            margin-left: 10px;
        }}
        .status-connected {{
            background: #10b981;
            color: #fff;
        }}
        .status-connecting {{
            background: #f59e0b;
            color: #fff;
        }}
        .status-disconnected {{
            background: #ef4444;
            color: #fff;
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
                <h1>
                    WKMP Audio Import
                    <span class="connection-status" id="connection-status">Connecting...</span>
                </h1>
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
    <script src=\"/static/wkmp-sse.js\"></script>
    <script>
        // Connect to SSE for connection status monitoring using shared WKMP utility
        if (typeof WkmpSSEConnection !== 'undefined') {{
            const sse = new WkmpSSEConnection('/events', 'connection-status');
            sse.connect();
        }} else {{
            console.error('WkmpSSEConnection class not found - wkmp-sse.js failed to load');
            const statusEl = document.getElementById('connection-status');
            if (statusEl) {{
                statusEl.className = 'connection-status status-disconnected';
                statusEl.textContent = 'Script Error';
            }}
        }}
    </script>
    </div>
</body>
</html>",
        html
    ))
}

