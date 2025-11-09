# 80s Techno Dance Club UI - High Level Specification

## Overview
Immersive late 1980s techno/alternative dance party interface. Think: strobing lights, VU meters, cassette deck aesthetics, neon grids, and early digital displays. Features animated visualizations synced to music and a cyberpunk/new wave aesthetic.

## Visual Theme References

### Design Inspiration
- **Nightclub Elements**: DJ booth, mixing console, laser grid floors, smoke effects
- **Technology**: Roland TR-808/909 drum machines, cassette decks, early samplers
- **Graphics**: Neon wireframe grids, CRT scanlines, early computer graphics
- **Culture**: Blade Runner aesthetics, Max Headroom glitch effects, MTV graphics
- **Colors**: Electric blue, hot pink, neon green, purple, black backgrounds

## Integration Points

### Backend Communication
```rust
// Same as jukebox - reuse existing backend
GET /dance-club/state
GET /dance-club/events (SSE)
POST /dance-club/enqueue

// Additional for visualizations
GET /dance-club/audio-analysis
→ { beat_detected, tempo_bpm, frequency_bands[8], energy_level }
```

### Extended Data Structures
```rust
struct SongMetadata {
    // Standard fields
    song_id: String,
    title: String,
    artist: String,
    
    // Extended for 80s aesthetic
    release_format: Option<String>,  // "12\" Single", "Cassette", "CD Single"
    remix_version: Option<String>,   // "Extended Mix", "Dub Version"
    tempo_bpm: Option<u16>,          // For beat-synced animations
}
```

## UI Concept: Virtual DJ Booth

### Main View - The Club
```
┌─────────────────────────────────────────────────┐
│  ╔══════════════════════════════════════════╗  │
│  ║  [LASER GRID CEILING WITH SCANLINES]     ║  │
│  ║  [ANIMATED AUDIENCE SILHOUETTES]         ║  │
│  ╚══════════════════════════════════════════╝  │
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │  ◆◆◆ NOW PLAYING ◆◆◆                     │ │
│  │  ARTIST: [Scrolling LED text]            │ │
│  │  TRACK:  [Scrolling LED text]            │ │
│  │  [████████░░] 3:42 / 6:15                │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
│  [VU METERS - 8 BANDS BOUNCING TO BEAT]       │
│  ██░ ███░ █████ ████░ ███░ ██████ ████ ███░   │
│                                                 │
│  [ROTATING VINYL/CASSETTE DECK ANIMATION]      │
│                                                 │
│  [STROBE/LASER EFFECTS OVERLAY]                │
└─────────────────────────────────────────────────┘
```

### Track Selection View - "The Crates"
```
┌─────────────────────────────────────────────────┐
│  ╔═══════════ RECORD CRATE ══════════════════╗ │
│  ║                                            ║ │
│  ║  [View from above: vinyl/cassettes        ║ │
│  ║   standing in milk crates, dividers       ║ │
│  ║   with genre labels]                      ║ │
│  ║                                            ║ │
│  ║  Click spine → flips to show track list   ║ │
│  ╚════════════════════════════════════════════╝ │
│                                                 │
│  Alternative: Cassette Wall                     │
│  ┌────┬────┬────┬────┬────┬────┐              │
│  │ T1 │ T2 │ T3 │ T4 │ T5 │ T6 │  [Cassette   │
│  ├────┼────┼────┼────┼────┼────┤   spines in  │
│  │ T7 │ T8 │ T9 │T10 │T11 │T12 │   shelves]   │
│  └────┴────┴────┴────┴────┴────┘              │
└─────────────────────────────────────────────────┘
```

## Asset Generation Strategy

### Visual Elements (AI Generated)

#### 1. Vinyl/Cassette Assets
**12" Single Records**:
- Plain black vinyl with center labels
- Label designs: Bold typography, geometric patterns
- Fluorescent/neon colored labels (hot pink, electric blue)
- Genre-specific label templates:
  - Techno/House: Geometric, minimal, sans-serif
  - Alternative: Distressed, punk aesthetic
  - New Wave: Art deco, stylized
  - Industrial: Harsh typography, distorted

**Cassette Tapes**:
- Classic cassette design with see-through shell
- J-card spine with artist/title in impact font
- Neon-colored shells and labels
- Visible tape spools that rotate when playing

**Generation Approach**:
```python
# Stable Diffusion prompt examples
vinyl_prompt = """
12 inch vinyl record label, 1980s techno music design,
geometric shapes, neon {color} on black background,
bold sans-serif typography, minimal aesthetic,
artist: {artist}, title: {title},
professional product photography, centered, high detail
"""

cassette_prompt = """
1980s cassette tape, {color} translucent plastic shell,
visible tape spools, white sticker label with bold text,
artist: {artist}, title: {title},
new wave aesthetic, studio photography, slight reflection
"""
```

#### 2. Background Elements
**Laser Grid Floor**:
- Tron-style wireframe perspective grid
- Neon lines: cyan, magenta, yellow
- Animated: pulses with beat, ripples on transitions
- Generated once, animated with shaders/CSS

**Dance Floor Crowd**:
- Silhouettes of dancing figures
- Simplified geometric shapes
- Animated with procedural motion
- Strobe lighting effects

**DJ Booth/Equipment**:
- Mixing console with faders and knobs
- Rack-mounted equipment with LED indicators
- Turntables or cassette decks
- VU meters and oscilloscopes

### Animation Effects

#### Beat-Synchronized
```javascript
// Triggered by audio analysis
onBeat() {
  - VU meters spike
  - Laser grid pulses
  - Strobe flash
  - Crowd silhouettes jump
  - Screen scanline glitch
}

// Continuous
animate() {
  - Vinyl rotates at correct RPM
  - Cassette spools turn
  - LED text scrolls
  - Laser beams sweep
  - Fog/smoke particles drift
}
```

#### Track Transition Animation
**Option A: Vinyl Flip**
- Current record lifts up
- Spins 180° (flips over to B-side aesthetic)
- New record appears underneath
- Drops down onto platter
- Duration: 1.5 seconds

**Option B: Cassette Auto-Reverse**
- Cassette pops up from deck
- Mechanical flip animation
- Clicks back into deck
- Spools begin turning
- Duration: 1.0 seconds

**Option C: Holographic Swap**
- Current media pixelates/glitches
- Max Headroom-style digital distortion
- New media materializes from scan lines
- Duration: 0.8 seconds

## UI Components

### View States

#### 1. Main Club View (Default)
**Layout**:
- Top 1/3: Animated background (crowd, lights, ceiling)
- Middle 1/3: Now playing display (LED-style)
- Bottom 1/3: Playback controls and visualizer

**Interactive Elements**:
- Tap crowd → zoom to dance floor view
- Tap deck → track selection
- Swipe left/right → skip tracks

#### 2. Track Selection View
**Crate Digging Mode**:
- 3D perspective view of record/cassette crate
- Pinch to zoom, drag to flip through
- Each item shows: spine with artist/title
- Tap to see track list on back/label

**Alternative - Grid Mode**:
- Grid of cassette/vinyl spines
- Virtual scroll through thousands
- Search bar: Terminal/DOS aesthetic
- Filters: Genre, BPM, Year

#### 3. Dance Floor View (Optional)
**Full Immersion**:
- Fullscreen visualization
- Particle effects (smoke, lasers)
- Beat-reactive animation
- Crowd silhouettes in foreground
- Exit: Swipe up/down

### Component Architecture (React)
```
src/
  components/
    DanceClubUI.jsx           # Main container
    ClubScene.jsx             # Background animation
    NowPlayingLED.jsx         # LED-style display
    VUMeter.jsx               # Frequency visualizer
    MediaDeck.jsx             # Vinyl/cassette animation
    CrateDigger.jsx           # Track selection
    StrobeEffect.jsx          # Beat-synced flashes
    LaserGrid.jsx             # WebGL floor effect
  
  effects/
    audioAnalyzer.js          # FFT/beat detection
    particleSystem.js         # Smoke/light particles
    glitchEffect.js           # CRT/Max Headroom distortion
  
  shaders/
    laserGrid.glsl            # Grid floor shader
    scanlines.glsl            # CRT effect
    bloom.glsl                # Neon glow
```

## Mobile Performance Strategies

### Critical Optimizations

1. **Adaptive Visual Quality**
   ```javascript
   const deviceTier = detectDeviceTier();
   
   if (deviceTier === 'low') {
     // Static background instead of animated
     // 2D canvas instead of WebGL
     // No particle effects
     // Reduced animation framerate (15fps)
   } else if (deviceTier === 'medium') {
     // Simplified WebGL effects
     // Limited particle count (50)
     // 30fps animations
   } else {
     // Full experience
     // Complex shaders
     // 60fps target
   }
   ```

2. **Effect Layering**
   ```javascript
   // Separate layers by update frequency
   layers: {
     background: 'static image with CSS', // Never updates
     grid: 'WebGL canvas 30fps',         // Medium frequency
     particles: 'Canvas 2D 60fps',       // High frequency
     ui: 'DOM elements',                 // Event-driven
   }
   ```

3. **Beat Detection Optimization**
   ```javascript
   // Use Web Audio API for efficient analysis
   const audioContext = new AudioContext();
   const analyser = audioContext.createAnalyser();
   analyser.fftSize = 256; // Lower = faster
   
   // Only analyze when visible
   if (document.hidden) pauseAnalysis();
   ```

4. **Shader Simplification**
   ```glsl
   // Mobile: simple grid with color
   // Desktop: grid + reflections + glow + fog
   
   #ifdef MOBILE
     vec3 color = gridColor;
   #else
     vec3 color = gridColor + reflection + glow;
     color = applyFog(color, depth);
   #endif
   ```

5. **Texture Atlases**
   ```javascript
   // Combine all cassette/vinyl textures into one
   // Reduces draw calls, faster on mobile GPU
   const atlas = new TextureAtlas([
     'vinyl_01', 'vinyl_02', ... 'vinyl_50',
     'cassette_01', 'cassette_02', ... 'cassette_50'
   ]);
   ```

6. **Lazy Load Non-Critical Effects**
   ```javascript
   // Load strobe/glitch effects only when needed
   const GlitchEffect = lazy(() => import('./effects/glitch'));
   
   // Disable expensive effects on battery saver mode
   if (navigator.getBattery) {
     battery.addEventListener('chargingchange', 
       (e) => e.charging ? enableEffects() : disableEffects()
     );
   }
   ```

7. **CSS Hardware Acceleration**
   ```css
   .laser-beam {
     transform: translate3d(0, 0, 0); /* Force GPU layer */
     will-change: transform;
   }
   
   .vu-meter-bar {
     transform: scaleY(var(--level));
     transition: transform 0.05s ease-out;
   }
   ```

8. **Audio Analysis Throttling**
   ```javascript
   // Update visualizations at reduced rate on mobile
   const updateInterval = isMobile ? 50 : 16; // 20fps vs 60fps
   
   setInterval(() => {
     const data = analyser.getByteFrequencyData();
     updateVUMeters(data);
     detectBeat(data);
   }, updateInterval);
   ```

9. **Precomputed Animations**
   ```javascript
   // Pre-render expensive animations as sprite sheets
   // Example: vinyl rotation at 33.33 RPM
   const vinylFrames = preRenderRotation(360, 60); // 60 frames
   
   // Playback is just image swapping
   function animateVinyl(time) {
     const frame = (time * 33.33 / 60) % 60;
     vinyl.texture = vinylFrames[Math.floor(frame)];
   }
   ```

10. **Progressive Enhancement**
    ```javascript
    // Feature detection
    const capabilities = {
      webgl: !!document.createElement('canvas').getContext('webgl'),
      audioAnalysis: 'AudioContext' in window,
      particleEffects: !isMobile || deviceTier > 'low',
      complexShaders: webgl && !isMobile,
    };
    
    // Enable features based on capability
    if (!capabilities.webgl) {
      // Use canvas 2D or CSS for all effects
    }
    ```

## Performance Budgets

### Target Metrics
- **Initial Load**: < 2 seconds to first visual
- **Animation Frame Rate**: 30fps minimum (60fps desktop)
- **Audio Analysis Latency**: < 50ms beat detection
- **Memory Usage**: < 100MB on mobile
- **Bundle Size**: < 400KB gzipped

### Visual Complexity Budget
```javascript
const effectBudget = {
  mobile_low: {
    particles: 0,
    shaders: ['simple_grid'],
    animations: ['vinyl_rotate'],
    quality: 'low'
  },
  
  mobile_high: {
    particles: 50,
    shaders: ['grid', 'scanlines'],
    animations: ['vinyl_rotate', 'vu_meters', 'led_scroll'],
    quality: 'medium'
  },
  
  desktop: {
    particles: 500,
    shaders: ['grid', 'scanlines', 'bloom', 'fog'],
    animations: 'all',
    quality: 'high'
  }
};
```

## Asset Generation Details

### Format-Specific Assets

#### Vinyl Records
```yaml
generation:
  prompt_base: "12 inch vinyl record label, 1980s techno design"
  styles:
    techno_minimal:
      colors: ["#00ffff", "#ff00ff", "#ffff00"]
      design: "Geometric shapes, grid patterns, bauhaus influence"
    
    acid_house:
      colors: ["#ffff00", "#000000"]
      design: "Smiley face, bold text, rave aesthetic"
    
    industrial:
      colors: ["#ff0000", "#000000", "#808080"]
      design: "Distorted typography, mechanical elements, rust"
    
    new_wave:
      colors: ["#ff1493", "#00ffff", "#ffffff"]
      design: "Art deco, geometric, memphis design movement"

assets:
  resolution: 2048x2048
  variants:
    - static: "For display in crate"
    - rotating: "With motion blur for playback"
  
  generation_time: ~20 seconds per label
  library_estimate: 10,000 songs × 20s = 55 hours
```

#### Cassette Tapes
```yaml
generation:
  components:
    shell: "AI generate clear/colored plastic cassette body"
    j_card: "Spine with title/artist in bold 80s fonts"
    label: "White sticker with typewriter-style text"
  
  colors:
    - clear_with_color_hubs: ["cyan", "magenta", "yellow"]
    - solid_shells: ["black", "white", "neon pink"]
  
  animation:
    - spools_rotate: "Sync with playback position"
    - tape_visible: "Shows through clear shell"
  
  generation_approach:
    1. Generate cassette shell template (once per color)
    2. Generate j-card spine per song (10s each)
    3. Composite together programmatically
```

### Background Scene Elements

#### Laser Grid Floor
```yaml
generation:
  method: "WebGL shader (no AI generation needed)"
  
  shader_features:
    - perspective_grid: "Receding into distance"
    - neon_lines: "Glowing cyan/magenta"
    - beat_pulse: "Expand/contract with tempo"
    - ripple_effect: "On track transitions"
  
  fallback: "Pre-rendered animated GIF/WebP"
```

#### Crowd Silhouettes
```yaml
generation:
  method: "AI generate + procedural animation"
  
  prompt: """
  Silhouette of dancing people in nightclub,
  geometric simplified shapes, black on transparent,
  1980s dance moves, multiple figures,
  flat design, high contrast
  """
  
  animation:
    - idle_dance: "Looping procedural motion"
    - beat_jump: "React to bass hits"
    - layers: "3 depth layers for parallax"
  
  generation: "Create 10 variations, cycle randomly"
```

#### DJ Equipment
```yaml
generation:
  items:
    - mixer: "Classic 4-channel DJ mixer"
    - turntables: "Technics SL-1200 style"
    - rack_gear: "Samplers, effects, sequencers"
    - lights: "LED meters, button indicators"
  
  prompt_template: """
  1980s DJ equipment {item}, professional studio photography,
  black background, dramatic lighting, neon accents,
  buttons and faders visible, LED indicators,
  retro futuristic aesthetic, high detail
  """
  
  interactivity:
    - VU meters: "React to frequency data"
    - buttons: "Light up on beat"
    - faders: "Move slightly during playback"
```

## Effects & Shaders

### WebGL Shaders (Desktop)

#### 1. Laser Grid Floor
```glsl
// Perspective grid with neon glow
varying vec2 vUv;
uniform float time;
uniform float beatIntensity;

void main() {
  vec2 pos = vUv * 20.0; // Grid density
  vec2 grid = abs(fract(pos - 0.5) - 0.5) / fwidth(pos);
  float line = min(grid.x, grid.y);
  
  // Neon glow
  vec3 color = vec3(0.0, 1.0, 1.0); // Cyan
  color *= (1.0 - min(line, 1.0));
  color += vec3(0.5, 0.0, 0.5) * beatIntensity; // Pink pulse
  
  // Fade to distance
  float fog = 1.0 - vUv.y * 0.8;
  color *= fog;
  
  gl_FragColor = vec4(color, 1.0);
}
```

#### 2. CRT Scanlines
```glsl
// Retro CRT monitor effect
uniform sampler2D tDiffuse;
uniform float time;
varying vec2 vUv;

void main() {
  vec2 uv = vUv;
  
  // Slight barrel distortion
  vec2 center = uv - 0.5;
  float dist = length(center);
  uv = 0.5 + center * (1.0 - 0.1 * dist * dist);
  
  vec4 color = texture2D(tDiffuse, uv);
  
  // Scanlines
  float scanline = sin(uv.y * 800.0) * 0.04;
  color.rgb -= scanline;
  
  // Vignette
  float vignette = smoothstep(0.8, 0.2, dist);
  color.rgb *= vignette;
  
  gl_FragColor = color;
}
```

#### 3. Neon Bloom
```glsl
// Glow effect for neon elements
uniform sampler2D tDiffuse;
varying vec2 vUv;

void main() {
  vec4 color = texture2D(tDiffuse, vUv);
  
  // Extract bright areas
  float brightness = dot(color.rgb, vec3(0.299, 0.587, 0.114));
  vec3 bloom = color.rgb * smoothstep(0.6, 1.0, brightness);
  
  // Blur and add back (simplified)
  vec3 blurred = bloom * 1.5;
  color.rgb += blurred;
  
  gl_FragColor = color;
}
```

### Canvas 2D Effects (Mobile Fallback)

```javascript
// Simplified effects for mobile
class MobileEffects {
  drawLaserGrid(ctx, width, height, beat) {
    // Simple static grid with CSS
    ctx.strokeStyle = `rgba(0, 255, 255, ${0.5 + beat * 0.5})`;
    ctx.lineWidth = 2;
    
    // Horizontal lines
    for (let y = height * 0.5; y < height; y += 40) {
      const scale = y / height;
      ctx.strokeRect(0, y, width, 0);
    }
  }
  
  drawVUMeter(ctx, x, y, level, color) {
    // Simple bar graph
    ctx.fillStyle = color;
    const height = level * 100;
    ctx.fillRect(x, y - height, 20, height);
  }
  
  drawScanlines(ctx, width, height) {
    // Horizontal lines
    ctx.strokeStyle = 'rgba(0, 0, 0, 0.1)';
    for (let y = 0; y < height; y += 4) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }
  }
}
```

## Implementation Phases

### Phase 1: Core UI & Assets (Week 1-2)
- React component structure
- SSE integration (reuse from jukebox)
- Static club scene layout
- Generate 50 test vinyl/cassette assets
- Basic CSS animations

### Phase 2: Audio Visualization (Week 2-3)
- Web Audio API integration
- FFT analysis for frequency bands
- Beat detection algorithm
- VU meter components
- Test with real audio

### Phase 3: WebGL Effects (Week 3-4)
- Laser grid shader
- Scanline/CRT shader  
- Particle system
- Mobile fallback (Canvas 2D)
- Performance optimization

### Phase 4: Track Selection (Week 4-5)
- Crate digging interface
- Virtual scrolling for 10k songs
- Search/filter with terminal aesthetic
- Touch gestures for mobile

### Phase 5: Transitions & Polish (Week 5-6)
- Vinyl flip animation
- Cassette auto-reverse animation
- Glitch effects
- Beat-synchronized events
- Cross-device testing

### Phase 6: Asset Generation (Week 6-8)
- Generate vinyl labels for full library
- Generate cassette j-cards
- Batch processing with priorities
- Quality control

### Phase 7: Mobile Optimization (Week 7-8)
- Device tier detection
- Adaptive quality system
- Battery-aware features
- Performance profiling

### Phase 8: Production (Week 9)
- Final polish
- Error handling
- Monitoring/analytics
- Documentation

## Technical Stack

### Frontend
```javascript
{
  "react": "^18",
  "three": "^0.160",           // WebGL (optional)
  "canvas-confetti": "^1.6",   // Particle effects
  "framer-motion": "^10",      // Animations
  "tone.js": "^14"             // Audio analysis (alternative to Web Audio API)
}
```

### AI Generation Tools
- **Stable Diffusion SDXL**: Vinyl/cassette assets
- **MidJourney** (alternative): Higher quality but slower
- **ControlNet**: Consistency across variations
- **Photoshop/GIMP**: Final compositing and text overlay

### Shaders & Effects
- **WebGL/Three.js**: Desktop
- **Canvas 2D**: Mobile
- **CSS Animations**: Fallback
- **GSAP**: Complex animation sequencing

## Key Design Decisions

✅ **Theme**: Late 1980s techno/alternative dance club  
✅ **Media**: Vinyl 12" singles & cassette tapes  
✅ **Visualization**: Beat-reactive VU meters, laser grid, strobes  
✅ **Selection**: Crate digging or cassette wall metaphor  
✅ **Animation**: Format-appropriate (vinyl flip, cassette reverse)  
✅ **Mobile**: Aggressive optimization, adaptive quality, fallbacks  

## Comparison with Jukebox UI

| Aspect | Jukebox (1970s) | Dance Club (1980s) |
|--------|-----------------|-------------------|
| **Format** | 45 RPM singles on jukebox | 12" vinyl + cassettes |
| **Selection** | Button grid | Crate digging |
| **Animation** | Record change with tonearm | Vinyl flip / cassette reverse |
| **Visual** | Warm, nostalgic, mechanical | Neon, digital, energetic |
| **Effects** | Subtle, realistic | Strobes, lasers, glitch |
| **Tempo** | Slow, deliberate | Fast, beat-reactive |
| **Colors** | Earth tones, chrome | Neon, black, electric |
| **Tech** | Analog, tubes | Digital, early computers |

## Integration Checklist

- [ ] Reuse backend SSE infrastructure from jukebox
- [ ] Add audio analysis endpoint for beat detection
- [ ] Determine if tempo/BPM available in metadata
- [ ] Set up Stable Diffusion for vinyl/cassette generation
- [ ] Test WebGL performance on target mobile devices
- [ ] Generate 10 test assets (5 vinyl, 5 cassette)
- [ ] Prototype laser grid shader
- [ ] Build VU meter component with real audio
- [ ] Design crate digging interaction pattern
- [ ] Profile mobile performance with effects enabled
