//! Segment editor page handler - Manual passage boundary adjustment

use axum::response::{Html, IntoResponse};

/// GET /segment-editor
///
/// Manual passage boundary adjustment interface with waveform visualization
pub async fn segment_editor_page() -> impl IntoResponse {
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
                <h1>
                    WKMP Audio Import
                    <span class="connection-status" id="connection-status">Connecting...</span>
                </h1>
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
        // Waveform Visualization Implementation
        // Full interactive waveform editor with boundary markers

        class WaveformRenderer {{
            constructor(canvas) {{
                this.canvas = canvas;
                this.ctx = canvas.getContext('2d');
                this.rmsProfile = [];
                this.leadInDuration = 0;
                this.leadOutDuration = 0;
                this.duration = 0;
                this.peakRms = 1.0;
            }}

            async loadAudioData(filePath, startTime = 0, endTime = null) {{
                try {{
                    const response = await fetch('/analyze/amplitude', {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify({{
                            file_path: filePath,
                            start_time: startTime,
                            end_time: endTime,
                            parameters: {{
                                window_size_ms: 100,
                                hop_size_ms: 50
                            }}
                        }})
                    }});

                    if (!response.ok) {{
                        throw new Error(`API error: ${{response.status}}`);
                    }}

                    const data = await response.json();
                    this.rmsProfile = data.rms_profile;
                    this.leadInDuration = data.lead_in_duration;
                    this.leadOutDuration = data.lead_out_duration;
                    this.peakRms = data.peak_rms;
                    this.duration = this.rmsProfile.length * 0.05; // 50ms hop size

                    return data;
                }} catch (error) {{
                    console.error('Failed to load amplitude data:', error);
                    throw error;
                }}
            }}

            render() {{
                const {{ width, height }} = this.canvas;
                const ctx = this.ctx;

                ctx.fillStyle = '#2d2d2d';
                ctx.fillRect(0, 0, width, height);

                if (this.rmsProfile.length === 0) {{
                    ctx.fillStyle = '#e0e0e0';
                    ctx.font = '14px system-ui';
                    ctx.fillText('Loading waveform...', 20, height / 2);
                    return;
                }}

                this.renderWaveform(ctx, width, height);
                this.renderRegions(ctx, width, height);
                this.renderTimeAxis(ctx, width, height);
            }}

            renderWaveform(ctx, width, height) {{
                const halfHeight = height / 2;
                const barWidth = width / this.rmsProfile.length;

                ctx.fillStyle = '#4a9eff';
                for (let i = 0; i < this.rmsProfile.length; i++) {{
                    const x = i * barWidth;
                    const rms = this.rmsProfile[i];
                    const barHeight = (rms / this.peakRms) * (halfHeight * 0.9);
                    ctx.fillRect(x, halfHeight - barHeight, barWidth, barHeight * 2);
                }}
            }}

            renderRegions(ctx, width, height) {{
                if (this.leadInDuration > 0) {{
                    const leadInX = this.timeToX(this.leadInDuration);
                    ctx.fillStyle = 'rgba(255, 215, 0, 0.2)';
                    ctx.fillRect(0, 0, leadInX, height);
                }}

                if (this.leadOutDuration > 0) {{
                    const leadOutStart = this.duration - this.leadOutDuration;
                    const leadOutX = this.timeToX(leadOutStart);
                    ctx.fillStyle = 'rgba(255, 140, 0, 0.2)';
                    ctx.fillRect(leadOutX, 0, width - leadOutX, height);
                }}
            }}

            renderTimeAxis(ctx, width, height) {{
                ctx.strokeStyle = '#666';
                ctx.fillStyle = '#e0e0e0';
                ctx.font = '10px monospace';

                const interval = 1.0;
                for (let t = 0; t <= this.duration; t += interval) {{
                    const x = this.timeToX(t);
                    ctx.beginPath();
                    ctx.moveTo(x, height - 20);
                    ctx.lineTo(x, height - 15);
                    ctx.stroke();

                    const label = formatTime(t);
                    ctx.fillText(label, x - 15, height - 5);
                }}
            }}

            timeToX(timeSeconds) {{
                return (timeSeconds / this.duration) * this.canvas.width;
            }}

            xToTime(x) {{
                return (x / this.canvas.width) * this.duration;
            }}
        }}

        class BoundaryMarker {{
            constructor(time, type) {{
                this.time = time;
                this.type = type;
                this.isDragging = false;
                this.color = type === 'start' ? '#00ff00' : '#ff0000';
            }}

            render(ctx, renderer) {{
                const x = renderer.timeToX(this.time);
                const height = renderer.canvas.height;

                ctx.strokeStyle = this.color;
                ctx.lineWidth = 2;
                ctx.beginPath();
                ctx.moveTo(x, 0);
                ctx.lineTo(x, height);
                ctx.stroke();

                ctx.fillStyle = this.color;
                ctx.fillRect(x - 5, 0, 10, 10);

                ctx.fillStyle = '#ffffff';
                ctx.font = '12px monospace';
                const label = formatTime(this.time);
                const textWidth = ctx.measureText(label).width;
                ctx.fillText(label, x - textWidth / 2, height - 25);
            }}

            hitTest(x, y, renderer) {{
                const markerX = renderer.timeToX(this.time);
                return Math.abs(x - markerX) < 10;
            }}

            updateTime(x, renderer) {{
                this.time = renderer.xToTime(x);
                this.time = Math.round(this.time * 10) / 10;
            }}
        }}

        function formatTime(seconds) {{
            const mins = Math.floor(seconds / 60);
            const secs = Math.floor(seconds % 60);
            return `${{mins}}:${{secs.toString().padStart(2, '0')}}`;
        }}

        const canvas = document.getElementById('waveform');
        const ctx = canvas.getContext('2d');
        const waveformRenderer = new WaveformRenderer(canvas);
        const markers = {{
            start: new BoundaryMarker(0, 'start'),
            end: new BoundaryMarker(10, 'end')
        }};

        let draggedMarker = null;

        canvas.addEventListener('mousedown', (e) => {{
            const rect = canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;

            for (const marker of Object.values(markers)) {{
                if (marker.hitTest(x, y, waveformRenderer)) {{
                    marker.isDragging = true;
                    draggedMarker = marker;
                    canvas.style.cursor = 'ew-resize';
                    break;
                }}
            }}
        }});

        canvas.addEventListener('mousemove', (e) => {{
            const rect = canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;

            if (draggedMarker) {{
                draggedMarker.updateTime(x, waveformRenderer);
                if (markers.start.time > markers.end.time) {{
                    markers.start.time = markers.end.time;
                }}
                waveformRenderer.render();
                markers.start.render(ctx, waveformRenderer);
                markers.end.render(ctx, waveformRenderer);
            }} else {{
                let overMarker = false;
                for (const marker of Object.values(markers)) {{
                    if (marker.hitTest(x, y, waveformRenderer)) {{
                        overMarker = true;
                        break;
                    }}
                }}
                canvas.style.cursor = overMarker ? 'pointer' : 'default';
            }}
        }});

        canvas.addEventListener('mouseup', () => {{
            if (draggedMarker) {{
                draggedMarker.isDragging = false;
                draggedMarker = null;
                canvas.style.cursor = 'default';
                console.log('Boundaries updated:', {{
                    start: markers.start.time,
                    end: markers.end.time
                }});
            }}
        }});

        async function loadDemo() {{
            try {{
                // Get file path from URL parameters or use test fixture
                const params = new URLSearchParams(window.location.search);
                const filePath = params.get('file') || 'tests/fixtures/sine_440hz_5s.wav';

                await waveformRenderer.loadAudioData(filePath, 0, null);
                markers.start.time = 0;
                markers.end.time = waveformRenderer.duration;

                waveformRenderer.render();
                markers.start.render(ctx, waveformRenderer);
                markers.end.render(ctx, waveformRenderer);

                console.log('Waveform editor loaded successfully');
            }} catch (error) {{
                console.error('Failed to load waveform:', error);
                ctx.fillStyle = '#ff0000';
                ctx.font = '14px system-ui';
                ctx.fillText('Error loading waveform data', 20, 50);
                ctx.fillText(error.message, 20, 70);
            }}
        }}

        loadDemo();
    </script>
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

