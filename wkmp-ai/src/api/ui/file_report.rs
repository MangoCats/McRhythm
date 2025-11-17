//! File classification report page handler
//!
//! **[AIA-CLASSIFY-UI-010]** File classification report UI (PLAN027)

use axum::response::{Html, IntoResponse};

/// GET /file-report
///
/// File classification report page showing all files discovered during SCANNING
pub async fn file_report_page() -> impl IntoResponse {
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
    <title>WKMP Audio Import - File Classification Report</title>
    <link rel="stylesheet" href="/static/wkmp-ui.css">
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
            padding: 20px;
        }}
        .container {{
            max-width: 1400px;
            margin: 0 auto;
        }}
        header {{
            background-color: #2a2a2a;
            border-bottom: 2px solid #4a9eff;
            padding: 20px;
            margin-bottom: 30px;
            border-radius: 8px;
        }}
        h1 {{
            font-size: 28px;
            color: #4a9eff;
            margin-bottom: 5px;
        }}
        .subtitle {{
            color: #888;
            font-size: 14px;
        }}
        .scanned-folder {{
            margin-top: 15px;
            padding: 10px;
            background: #1a1a1a;
            border-radius: 4px;
            font-family: 'Courier New', monospace;
            color: #4a9eff;
        }}

        /* Category tabs */
        .category-tabs {{
            display: flex;
            gap: 10px;
            margin-bottom: 20px;
            border-bottom: 2px solid #3a3a3a;
        }}
        .tab {{
            padding: 15px 25px;
            background: #2a2a2a;
            border: 1px solid #3a3a3a;
            border-bottom: none;
            cursor: pointer;
            transition: all 0.3s;
            border-radius: 8px 8px 0 0;
            position: relative;
            top: 2px;
        }}
        .tab:hover {{
            background: #333;
        }}
        .tab.active {{
            background: #3a3a3a;
            border-color: #4a9eff;
            border-bottom-color: #3a3a3a;
        }}
        .tab-label {{
            font-weight: bold;
            color: #e0e0e0;
        }}
        .tab-count {{
            color: #888;
            font-size: 12px;
            margin-top: 5px;
        }}
        .tab-size {{
            color: #4a9eff;
            font-size: 11px;
        }}

        /* File list container */
        .file-list-container {{
            background: #2a2a2a;
            border-radius: 8px;
            overflow: hidden;
            min-height: 500px;
        }}
        .file-list-header {{
            display: grid;
            grid-template-columns: 3fr 1fr 1fr;
            padding: 15px 20px;
            background: #333;
            font-weight: bold;
            border-bottom: 2px solid #4a9eff;
            position: sticky;
            top: 0;
            z-index: 10;
        }}
        .file-list {{
            max-height: 600px;
            overflow-y: auto;
        }}
        .file-item {{
            display: grid;
            grid-template-columns: 3fr 1fr 1fr;
            padding: 12px 20px;
            border-bottom: 1px solid #333;
            transition: background 0.2s;
        }}
        .file-item:hover {{
            background: #333;
        }}
        .file-path {{
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            color: #e0e0e0;
            font-family: 'Courier New', monospace;
            font-size: 13px;
        }}
        .file-size {{
            text-align: right;
            color: #4a9eff;
            font-family: 'Courier New', monospace;
        }}
        .file-modified {{
            text-align: right;
            color: #888;
            font-size: 12px;
        }}

        /* Loading and error states */
        .loading {{
            text-align: center;
            padding: 50px;
            color: #888;
        }}
        .error {{
            text-align: center;
            padding: 50px;
            color: #ff6b6b;
        }}
        .empty-state {{
            text-align: center;
            padding: 50px;
            color: #888;
        }}

        /* Actions */
        .actions {{
            margin-top: 30px;
            display: flex;
            gap: 15px;
            justify-content: center;
        }}
        .btn {{
            padding: 12px 30px;
            font-size: 16px;
            cursor: pointer;
            border: none;
            border-radius: 6px;
            transition: all 0.3s;
            text-decoration: none;
            display: inline-block;
        }}
        .btn-primary {{
            background: #4a9eff;
            color: #fff;
        }}
        .btn-primary:hover {{
            background: #3a8eef;
        }}
        .btn-secondary {{
            background: #555;
            color: #e0e0e0;
        }}
        .btn-secondary:hover {{
            background: #666;
        }}

        /* Build info */
        .build-info {{
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #3a3a3a;
            text-align: center;
            color: #666;
            font-size: 12px;
            font-family: 'Courier New', monospace;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>File Classification Report</h1>
            <div class="subtitle">Complete inventory of all files discovered during scan</div>
            <div id="scanned-folder" class="scanned-folder"></div>
        </header>

        <div class="category-tabs" id="categoryTabs">
            <!-- Tabs will be dynamically generated -->
        </div>

        <div class="file-list-container">
            <div id="fileListContent">
                <div class="loading">Loading classification data...</div>
            </div>
        </div>

        <div class="actions">
            <a href="/import-progress" class="btn btn-primary">Continue to Processing</a>
            <button class="btn btn-secondary" onclick="cancelImport()">Cancel Import</button>
        </div>

        <div class="build-info">
            wkmp-ai v{} [{}] | Built {} ({})
        </div>
    </div>

    <script src="/static/file-report.js"></script>
</body>
</html>
"#, version, git_hash, build_timestamp, build_profile);

    Html(html)
}
