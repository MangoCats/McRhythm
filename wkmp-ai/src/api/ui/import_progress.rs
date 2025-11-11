//! Import progress page handler - Real-time progress tracking with SSE

use axum::response::{Html, IntoResponse};

/// GET /import-progress
///
/// Import progress page with real-time SSE updates
pub async fn import_progress_page() -> impl IntoResponse {
    // Get platform-appropriate default root folder path [REQ-NF-033]
    let default_root = wkmp_common::config::get_default_root_folder();
    let default_root_str = default_root.to_string_lossy();
    // Escape backslashes for JavaScript string
    let default_root_escaped = default_root_str.replace("\\", "\\\\");

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
    <link rel="stylesheet" href="/static/wkmp-ui.css">
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

        /* **[AIA-SEC-030]** AcoustID API Key Modal */
        .modal {{
            display: none;
            position: fixed;
            z-index: 1000;
            left: 0;
            top: 0;
            width: 100%;
            height: 100%;
            background-color: rgba(0, 0, 0, 0.7);
            justify-content: center;
            align-items: center;
        }}
        .modal-content {{
            background-color: #2a2a2a;
            padding: 30px;
            border-radius: 8px;
            max-width: 500px;
            width: 90%;
            border: 1px solid #4a9eff;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.5);
        }}
        .modal-header {{
            color: #4a9eff;
            font-size: 22px;
            margin-bottom: 15px;
        }}
        .modal-body {{
            margin-bottom: 20px;
        }}
        .modal-error {{
            background: #3a2a2a;
            color: #ff6b6b;
            padding: 10px;
            border-radius: 4px;
            margin-bottom: 15px;
            border: 1px solid #5a3a3a;
        }}
        .modal-input {{
            width: 100%;
            padding: 10px;
            background: #1a1a1a;
            border: 1px solid #3a3a3a;
            border-radius: 4px;
            color: #e0e0e0;
            font-size: 14px;
            margin-bottom: 15px;
        }}
        .modal-input:focus {{
            outline: none;
            border-color: #4a9eff;
        }}
        .modal-buttons {{
            display: flex;
            gap: 10px;
            justify-content: flex-end;
        }}
        .modal-button {{
            padding: 10px 20px;
            border: none;
            border-radius: 4px;
            font-size: 14px;
            cursor: pointer;
            transition: all 0.2s;
        }}
        .modal-button-primary {{
            background: #4a9eff;
            color: #fff;
        }}
        .modal-button-primary:hover {{
            background: #3a8eef;
        }}
        .modal-button-primary:disabled {{
            background: #2a5a8f;
            cursor: not-allowed;
        }}
        .modal-button-secondary {{
            background: #3a3a3a;
            color: #e0e0e0;
        }}
        .modal-button-secondary:hover {{
            background: #4a4a4a;
        }}
        .modal-button-secondary:disabled {{
            background: #2a2a2a;
            cursor: not-allowed;
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

    <!-- **[AIA-SEC-030]** AcoustID API Key Validation Modal -->
    <div id="acoustid-modal" class="modal">
        <div class="modal-content">
            <div class="modal-header">AcoustID API Key Required</div>
            <div class="modal-body">
                <div class="modal-error" id="acoustid-error-message"></div>
                <p>The import process requires a valid AcoustID API key for music identification. You can:</p>
                <ol>
                    <li><strong>Enter a valid API key</strong> - Get one free at <a href="https://acoustid.org/new-application" target="_blank" style="color: #4a9eff;">acoustid.org/new-application</a></li>
                    <li><strong>Skip AcoustID</strong> - Continue import without fingerprint-based identification (reduced accuracy)</li>
                </ol>
                <input
                    type="text"
                    id="acoustid-api-key"
                    class="modal-input"
                    placeholder="Enter AcoustID API key..."
                    onkeypress="if(event.key==='Enter') submitAcoustIDKey()"
                />
                <div class="modal-error" id="acoustid-modal-error" style="display: none;"></div>
            </div>
            <div class="modal-buttons">
                <button
                    id="acoustid-skip-btn"
                    class="modal-button modal-button-secondary"
                    onclick="skipAcoustID()"
                >Skip AcoustID</button>
                <button
                    id="acoustid-submit-btn"
                    class="modal-button modal-button-primary"
                    onclick="submitAcoustIDKey()"
                >Submit Key</button>
            </div>
        </div>
    </div>

    <script src="/static/wkmp-sse.js"></script>
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
        "#, version, &git_hash[..8], build_profile, build_timestamp, &default_root_escaped);

    Html(html)
}

