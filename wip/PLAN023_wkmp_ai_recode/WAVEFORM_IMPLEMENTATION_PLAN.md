# Waveform Visualization Implementation Plan

**Date:** 2025-11-09
**Feature:** Interactive waveform rendering with boundary markers
**Status:** Planning Complete → Ready for Implementation

---

## Requirements

### Functional Requirements

1. **Waveform Rendering**
   - Fetch amplitude data from `/analyze/amplitude` API
   - Render RMS envelope on HTML5 canvas
   - Display waveform with proper scaling
   - Show lead-in/lead-out regions visually

2. **Boundary Markers**
   - Display passage start/end boundaries as vertical lines
   - Visual distinction for different marker types
   - Interactive dragging to adjust boundaries
   - Snap-to-grid behavior for precise positioning

3. **User Interaction**
   - Click and drag markers
   - Visual feedback during drag
   - Save changes automatically or on confirm
   - Keyboard shortcuts for fine adjustment

### Non-Functional Requirements

1. **Performance**
   - Smooth rendering (60 FPS)
   - Efficient canvas updates (only redraw on change)
   - Handle long audio files (10+ minutes)

2. **Usability**
   - Clear visual feedback
   - Responsive design
   - Error handling for missing data

---

## API Integration

### Amplitude Analysis Endpoint

**POST** `/analyze/amplitude`

**Request:**
```json
{
  "file_path": "/path/to/audio.mp3",
  "start_time": 0.0,
  "end_time": null,
  "parameters": {
    "window_size_ms": 100,
    "hop_size_ms": 50
  }
}
```

**Response:**
```json
{
  "file_path": "/path/to/audio.mp3",
  "peak_rms": 0.85,
  "lead_in_duration": 2.5,
  "lead_out_duration": 1.8,
  "quick_ramp_up": true,
  "quick_ramp_down": false,
  "rms_profile": [0.1, 0.3, 0.5, ..., 0.2],
  "analyzed_at": "2025-11-09T..."
}
```

**RMS Profile:**
- Array of RMS values (0.0 - 1.0+)
- One value per window (default: 100ms windows with 50ms hop)
- Linear scale (not dB)

---

## Implementation Design

### Architecture

```
┌─────────────────────────────────────────┐
│           UI (segment_editor)           │
│  ┌───────────────────────────────────┐  │
│  │     Canvas (1200x200)             │  │
│  │  ┌─────────────────────────────┐  │  │
│  │  │  Waveform Rendering Layer   │  │  │
│  │  └─────────────────────────────┘  │  │
│  │  ┌─────────────────────────────┐  │  │
│  │  │  Boundary Markers Layer     │  │  │
│  │  └─────────────────────────────┘  │  │
│  │  ┌─────────────────────────────┐  │  │
│  │  │  Interaction Layer          │  │  │
│  │  └─────────────────────────────┘  │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
         ↓ HTTP POST
┌─────────────────────────────────────────┐
│      /analyze/amplitude API             │
│  (AmplitudeAnalyzer service)            │
└─────────────────────────────────────────┘
```

### Component Structure

1. **WaveformRenderer** (JavaScript class)
   - Fetch amplitude data
   - Render waveform on canvas
   - Handle canvas resizing
   - Provide coordinate conversion (time ↔ pixels)

2. **BoundaryMarker** (JavaScript class)
   - Represent single boundary (start/end)
   - Render marker line
   - Handle drag interactions
   - Snap to grid

3. **SegmentEditor** (JavaScript class)
   - Coordinate waveform + markers
   - Handle user interactions
   - Save boundary changes
   - Manage editor state

---

## Implementation Steps

### Phase 1: Waveform Rendering (Core)

**File:** `src/api/ui.rs` (inline JavaScript in HTML template)

1. **Create WaveformRenderer class**
   ```javascript
   class WaveformRenderer {
       constructor(canvas, audioFilePath, startTime, endTime) {
           this.canvas = canvas;
           this.ctx = canvas.getContext('2d');
           this.audioFilePath = audioFilePath;
           this.startTime = startTime;
           this.endTime = endTime;
           this.rmsProfile = [];
           this.leadInDuration = 0;
           this.leadOutDuration = 0;
       }

       async fetchAmplitudeData() {
           // POST to /analyze/amplitude
           // Store rms_profile, lead_in_duration, lead_out_duration
       }

       render() {
           // Clear canvas
           // Draw waveform from rmsProfile
           // Highlight lead-in/lead-out regions
       }

       timeToX(timeSeconds) {
           // Convert time to canvas X coordinate
       }

       xToTime(x) {
           // Convert canvas X to time in seconds
       }
   }
   ```

2. **Render RMS envelope**
   - Draw as filled area (0 to RMS value)
   - Use gradient fill (blue for normal, yellow for lead-in, orange for lead-out)
   - Scale RMS values to canvas height

3. **Visual styling**
   - Background: dark gray (#2d2d2d)
   - Waveform: blue (#4a9eff)
   - Lead-in region: yellow tint (#ffd700 overlay)
   - Lead-out region: orange tint (#ff8c00 overlay)

### Phase 2: Boundary Markers

1. **Create BoundaryMarker class**
   ```javascript
   class BoundaryMarker {
       constructor(time, type, color) {
           this.time = time;  // seconds
           this.type = type;  // 'start' or 'end'
           this.color = color;
           this.isDragging = false;
       }

       render(ctx, renderer) {
           const x = renderer.timeToX(this.time);
           // Draw vertical line
           // Draw handle at top/bottom
           // Draw time label
       }

       hitTest(x, y, renderer) {
           // Check if click is within marker hit area
       }
   }
   ```

2. **Marker styling**
   - Start marker: green (#00ff00)
   - End marker: red (#ff0000)
   - Line width: 2px
   - Handle: 10x10 square at top
   - Label: time in MM:SS format

### Phase 3: Interaction

1. **Mouse events**
   ```javascript
   canvas.addEventListener('mousedown', (e) => {
       // Check if clicking on marker handle
       // Start dragging
   });

   canvas.addEventListener('mousemove', (e) => {
       // Update marker position if dragging
       // Show cursor feedback (resize cursor over marker)
   });

   canvas.addEventListener('mouseup', (e) => {
       // Stop dragging
       // Save new boundary position
   });
   ```

2. **Drag behavior**
   - Constrain to canvas bounds
   - Snap to 0.1 second intervals
   - Start marker cannot exceed end marker
   - Visual feedback during drag (semi-transparent line)

### Phase 4: Integration

1. **URL parameters**
   - Add query params: `?file=...&start=...&end=...`
   - Extract from URL on page load
   - Initialize waveform with these values

2. **Save changes**
   - POST to `/passages/{id}/boundaries` API
   - Or use browser storage for demo
   - Show success/error feedback

---

## Code Implementation

### Inline JavaScript (src/api/ui.rs)

Replace the TODO section (lines 893-897) with full implementation:

```javascript
// Waveform Visualization Implementation
class WaveformRenderer {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.rmsProfile = [];
        this.leadInDuration = 0;
        this.leadOutDuration = 0;
        this.duration = 0;
        this.peakRms = 1.0;
    }

    async loadAudioData(filePath, startTime = 0, endTime = null) {
        try {
            const response = await fetch('/analyze/amplitude', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    file_path: filePath,
                    start_time: startTime,
                    end_time: endTime,
                    parameters: {
                        window_size_ms: 100,
                        hop_size_ms: 50
                    }
                })
            });

            if (!response.ok) {
                throw new Error(`API error: ${response.status}`);
            }

            const data = await response.json();
            this.rmsProfile = data.rms_profile;
            this.leadInDuration = data.lead_in_duration;
            this.leadOutDuration = data.lead_out_duration;
            this.peakRms = data.peak_rms;

            // Calculate duration from RMS profile
            // Assuming 50ms hop size
            this.duration = this.rmsProfile.length * 0.05;

            return data;
        } catch (error) {
            console.error('Failed to load amplitude data:', error);
            throw error;
        }
    }

    render() {
        const { width, height } = this.canvas;
        const ctx = this.ctx;

        // Clear canvas
        ctx.fillStyle = '#2d2d2d';
        ctx.fillRect(0, 0, width, height);

        if (this.rmsProfile.length === 0) {
            // No data loaded yet
            ctx.fillStyle = '#e0e0e0';
            ctx.font = '14px system-ui';
            ctx.fillText('Loading waveform...', 20, height / 2);
            return;
        }

        // Draw waveform envelope
        this.renderWaveform(ctx, width, height);

        // Draw lead-in/lead-out regions
        this.renderRegions(ctx, width, height);

        // Draw time axis
        this.renderTimeAxis(ctx, width, height);
    }

    renderWaveform(ctx, width, height) {
        const halfHeight = height / 2;
        const barWidth = width / this.rmsProfile.length;

        // Draw waveform as filled bars
        ctx.fillStyle = '#4a9eff';
        ctx.beginPath();

        for (let i = 0; i < this.rmsProfile.length; i++) {
            const x = i * barWidth;
            const rms = this.rmsProfile[i];
            const barHeight = (rms / this.peakRms) * (halfHeight * 0.9);

            // Draw symmetric waveform (top and bottom)
            ctx.fillRect(x, halfHeight - barHeight, barWidth, barHeight * 2);
        }
    }

    renderRegions(ctx, width, height) {
        // Highlight lead-in region
        if (this.leadInDuration > 0) {
            const leadInX = this.timeToX(this.leadInDuration);
            ctx.fillStyle = 'rgba(255, 215, 0, 0.2)'; // Yellow overlay
            ctx.fillRect(0, 0, leadInX, height);
        }

        // Highlight lead-out region
        if (this.leadOutDuration > 0) {
            const leadOutStart = this.duration - this.leadOutDuration;
            const leadOutX = this.timeToX(leadOutStart);
            ctx.fillStyle = 'rgba(255, 140, 0, 0.2)'; // Orange overlay
            ctx.fillRect(leadOutX, 0, width - leadOutX, height);
        }
    }

    renderTimeAxis(ctx, width, height) {
        ctx.strokeStyle = '#666';
        ctx.fillStyle = '#e0e0e0';
        ctx.font = '10px monospace';

        // Draw time markers every second
        const interval = 1.0; // 1 second
        for (let t = 0; t <= this.duration; t += interval) {
            const x = this.timeToX(t);

            // Draw tick
            ctx.beginPath();
            ctx.moveTo(x, height - 20);
            ctx.lineTo(x, height - 15);
            ctx.stroke();

            // Draw time label
            const label = formatTime(t);
            ctx.fillText(label, x - 15, height - 5);
        }
    }

    timeToX(timeSeconds) {
        return (timeSeconds / this.duration) * this.canvas.width;
    }

    xToTime(x) {
        return (x / this.canvas.width) * this.duration;
    }
}

class BoundaryMarker {
    constructor(time, type) {
        this.time = time;
        this.type = type; // 'start' or 'end'
        this.isDragging = false;
        this.color = type === 'start' ? '#00ff00' : '#ff0000';
    }

    render(ctx, renderer) {
        const x = renderer.timeToX(this.time);
        const height = renderer.canvas.height;

        // Draw vertical line
        ctx.strokeStyle = this.color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();

        // Draw handle at top
        ctx.fillStyle = this.color;
        ctx.fillRect(x - 5, 0, 10, 10);

        // Draw time label
        ctx.fillStyle = '#ffffff';
        ctx.font = '12px monospace';
        const label = formatTime(this.time);
        const textWidth = ctx.measureText(label).width;
        ctx.fillText(label, x - textWidth / 2, height - 25);
    }

    hitTest(x, y, renderer) {
        const markerX = renderer.timeToX(this.time);
        // Check if within +/- 10px of marker
        return Math.abs(x - markerX) < 10;
    }

    updateTime(x, renderer) {
        this.time = renderer.xToTime(x);
        // Snap to 0.1 second intervals
        this.time = Math.round(this.time * 10) / 10;
    }
}

// Helper function
function formatTime(seconds) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
}

// Initialize waveform editor
const waveformRenderer = new WaveformRenderer(canvas);
const markers = {
    start: new BoundaryMarker(0, 'start'),
    end: new BoundaryMarker(10, 'end') // Placeholder, will be set from data
};

let draggedMarker = null;

// Mouse event handlers
canvas.addEventListener('mousedown', (e) => {
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Check if clicking on a marker
    for (const marker of Object.values(markers)) {
        if (marker.hitTest(x, y, waveformRenderer)) {
            marker.isDragging = true;
            draggedMarker = marker;
            canvas.style.cursor = 'ew-resize';
            break;
        }
    }
});

canvas.addEventListener('mousemove', (e) => {
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (draggedMarker) {
        // Update marker position
        draggedMarker.updateTime(x, waveformRenderer);

        // Constrain markers
        if (markers.start.time > markers.end.time) {
            markers.start.time = markers.end.time;
        }

        // Redraw
        waveformRenderer.render();
        markers.start.render(ctx, waveformRenderer);
        markers.end.render(ctx, waveformRenderer);
    } else {
        // Update cursor based on hover
        let overMarker = false;
        for (const marker of Object.values(markers)) {
            if (marker.hitTest(x, y, waveformRenderer)) {
                overMarker = true;
                break;
            }
        }
        canvas.style.cursor = overMarker ? 'pointer' : 'default';
    }
});

canvas.addEventListener('mouseup', () => {
    if (draggedMarker) {
        draggedMarker.isDragging = false;
        draggedMarker = null;
        canvas.style.cursor = 'default';

        console.log('Boundaries updated:', {
            start: markers.start.time,
            end: markers.end.time
        });

        // TODO: Save to backend
    }
});

// Load demo data
async function loadDemo() {
    try {
        // For demo, use test fixture
        const filePath = '/path/to/test/audio.wav';
        await waveformRenderer.loadAudioData(filePath, 0, null);

        // Set initial markers
        markers.start.time = 0;
        markers.end.time = waveformRenderer.duration;

        // Initial render
        waveformRenderer.render();
        markers.start.render(ctx, waveformRenderer);
        markers.end.render(ctx, waveformRenderer);
    } catch (error) {
        console.error('Failed to load demo:', error);
        // Show error message on canvas
        ctx.fillStyle = '#ff0000';
        ctx.font = '14px system-ui';
        ctx.fillText('Error loading waveform data', 20, 50);
        ctx.fillText(error.message, 20, 70);
    }
}

// Initialize on page load
loadDemo();
```

---

## Testing Plan

1. **Unit Tests (Manual)**
   - Test with 5-second sine wave fixture
   - Test with 30-second music file
   - Test with file having distinct lead-in/lead-out

2. **Integration Tests**
   - Load waveform from API
   - Drag markers and verify coordinates
   - Verify marker constraints (start < end)

3. **UI/UX Tests**
   - Responsive canvas sizing
   - Cursor feedback
   - Visual clarity of markers and regions

---

## Estimated Effort

- Planning: 30 min (COMPLETE)
- Core waveform rendering: 1 hour
- Boundary markers: 1 hour
- Interaction logic: 1 hour
- Integration & testing: 30 min
- **Total:** ~3.5 hours

---

## Success Criteria

- ✅ Waveform loads from amplitude API
- ✅ RMS envelope renders correctly
- ✅ Lead-in/lead-out regions highlighted
- ✅ Boundary markers draggable
- ✅ Time labels display correctly
- ✅ Smooth interaction (no lag)
- ✅ Works with test fixtures

---

## Next Steps

1. Implement WaveformRenderer class
2. Implement BoundaryMarker class
3. Add mouse event handlers
4. Test with audio fixtures
5. Document usage in UI

---

**Status:** READY FOR IMPLEMENTATION
