//! Import complete page handler - Completion summary with return link

use axum::response::{Html, IntoResponse};

/// GET /import-complete
///
/// Import completion summary page with return link to wkmp-ui
pub async fn import_complete_page() -> impl IntoResponse {
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
                <h1>
                    WKMP Audio Import
                    <span class="connection-status" id="connection-status">Connecting...</span>
                </h1>
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
    <script src=\"/static/wkmp-sse.js\"></script>
    <script>
        // Connect to SSE for connection status monitoring using shared WKMP utility
        const sse = new WkmpSSEConnection('/events', 'connection-status');
        sse.connect();
    </script>
    </div>
</body>
</html>
        "#, version, &git_hash[..8], build_profile, build_timestamp
    ))
}

