# Jukebox UI - High Level Specification

## Overview
Visual jukebox interface for music player system. Shows currently playing song, queue, and allows user song selection. Features animated record-changing visualization and 1970s-style aesthetic.

## Integration Points

### Backend Communication
```rust
// Initial page load
GET /jukebox/state
→ { currently_playing, queue, catalog_summary }

// Real-time updates
GET /jukebox/events (SSE)
→ event: now_playing
→ event: playback_progress
→ event: queue_update
→ event: song_transition

// User actions
POST /jukebox/enqueue
{ song_id, user_selected: true }
```

### Data Structures
```rust
struct SongMetadata {
    song_id: String,
    title: String,
    artist: String,
    album: Option<String>,
    year: Option<u16>,
    musicbrainz: Option<MusicBrainzData>, // For label identification
}

struct MusicBrainzData {
    recording_mbid: String,
    label: Option<String>,        // "Capitol Records", "Atlantic", etc.
    catalog_number: Option<String>,
}
```

## Asset Generation Strategy

### Record Labels (AI Generated)
- **Source**: Stable Diffusion SDXL + ControlNet
- **Style**: Authentic 1970s record label designs (Capitol, Atlantic, Columbia, Warner Bros, Motown, etc.)
- **Approach**: 
  - Use MusicBrainz label data to determine authentic design template
  - Generate base label with AI using reference images from web search
  - Add crisp text overlay programmatically (PIL/Canvas)
- **Generation Time**: ~30 seconds per label with GPU
- **Library Size**: 10,000 songs
- **Progressive Strategy**:
  - Priority 1: Currently playing + next 3 in queue (on-demand)
  - Priority 2: Top 1000 most played (background generation)
  - Priority 3: Full catalog (over 3-5 days)

### Animation Assets (Hybrid)
**Static elements** (pre-rendered once):
- Turntable base
- Tonearm in 12 positions
- Lighting/shadow effects

**Dynamic elements** (per-song):
- Record with generated label
- Motion blur variants (generated on-demand)

**Composition**: Real-time HTML5 Canvas/WebGL layering at 30fps

### Asset Storage Structure
```
assets/
  jukebox/
    base/
      jukebox_full.png           # Original jukebox image
      regions.json               # Clickable region coordinates
    
    turntable/
      base.png
      tonearm_pos_{00-11}.png
      effects/
    
    records/
      labels/
        {song_id}.png            # 2048x2048 generated labels
      
      composed/
        {song_id}_record.png     # Label on vinyl (lazy generated)
    
    selection_panel/
      button_states/             # UI button templates
      thumbnails/                # Song thumbnail overlays
```

## UI Components

### Three View States

1. **Full Jukebox View** (Default)
   - Display entire jukebox image as background
   - Overlay now-playing info on display area
   - Highlight selection panel on hover
   - Click panel → zoom to selection view

2. **Selection Panel View** (Zoomed)
   - 10x10 grid of illuminated buttons (100 songs per page)
   - Virtual scroll for 10,000 song library
   - Each button shows: artist, title, duration
   - Click button → POST /enqueue → return to full view
   - Search/filter controls at top
   - Back button → return to full view

3. **Record Change Animation** (Overlay)
   - Triggered by SSE `song_transition` event
   - 2.5 second sequence:
     - Tonearm lifts (0.5s)
     - Records slide left/right (0.5s)
     - New record settles (0.7s)
     - Tonearm descends and touches down (0.8s)
   - Composited in real-time from static + dynamic assets
   - Fallback: Simple crossfade if assets not ready

### Component Architecture (React)
```
src/
  components/
    JukeboxUI.jsx              # Main container
    FullView.jsx               # Default jukebox display
    SelectionPanel.jsx         # Song selection grid
    AnimationOverlay.jsx       # Record change animation (Canvas)
    NowPlaying.jsx             # Current song display
    QueueDisplay.jsx           # Upcoming songs
  
  hooks/
    useSSE.js                  # SSE connection management
    useAssetPreloader.js       # Preload next song assets
  
  services/
    api.js                     # HTTP/SSE client
    assetManager.js            # Cache and load assets
    animationEngine.js         # Canvas rendering
```

## Mobile Performance Strategies

### Critical Optimizations

1. **Responsive Asset Loading**
   ```javascript
   // Detect device and load appropriate quality
   const quality = window.innerWidth < 768 ? 'mobile' : 'desktop';
   const assetRes = {
     mobile: 512,    // 512x512 images
     desktop: 2048   // 2048x2048 images
   };
   ```

2. **Virtual Scrolling**
   - Only render visible buttons in selection panel
   - Lazy load thumbnails as user scrolls
   - Use `IntersectionObserver` for visibility detection
   - Recycle DOM elements for off-screen items

3. **Animation Performance**
   ```javascript
   // Use CSS transforms for GPU acceleration
   transform: translate3d(x, y, 0);
   will-change: transform, opacity;
   
   // Or WebGL for complex compositing
   // Fallback to simple CSS transitions on low-end devices
   ```

4. **Asset Preloading**
   ```javascript
   // When song is 30s from ending
   preloadAssets(nextSongId, quality);
   
   // Preload strategy:
   // - Next 3 in queue: base images only
   // - Motion blur: generate on-demand in Web Worker
   ```

5. **Memory Management**
   ```javascript
   // LRU cache with size limits
   const cacheLimit = isMobile ? 50 * 1024 * 1024 : 200 * 1024 * 1024; // 50MB mobile, 200MB desktop
   
   // Aggressively unload off-screen assets
   // Keep only: current playing, next 3 in queue, selection page
   ```

6. **Network Optimization**
   ```javascript
   // Use WebP format with fallback
   <picture>
     <source srcset="record.webp" type="image/webp">
     <img src="record.png" alt="Record">
   </picture>
   
   // HTTP/2 server push for critical assets
   // Service Worker for offline support
   ```

7. **Progressive Enhancement**
   ```javascript
   // Feature detection
   const canAnimate = 
     window.requestAnimationFrame && 
     !window.matchMedia('(prefers-reduced-motion: reduce)').matches &&
     performance.now() > 0;
   
   if (!canAnimate) {
     // Static images only, no animations
   }
   ```

8. **Touch Optimization**
   ```css
   /* Larger touch targets */
   .selection-button {
     min-width: 44px;
     min-height: 44px;
     touch-action: manipulation; /* Disable double-tap zoom */
   }
   
   /* Smooth scrolling */
   -webkit-overflow-scrolling: touch;
   ```

9. **Reduce Layout Thrashing**
   ```javascript
   // Batch DOM reads and writes
   requestAnimationFrame(() => {
     // Read phase
     const rect = element.getBoundingClientRect();
     
     // Write phase
     element.style.transform = `translate(${x}px, ${y}px)`;
   });
   ```

10. **Code Splitting**
    ```javascript
    // Lazy load animation engine only when needed
    const AnimationEngine = lazy(() => import('./animationEngine'));
    
    // Preload on interaction
    onMouseEnter={() => import('./animationEngine')};
    ```

## Performance Budgets

### Target Metrics (Mobile)
- **Initial Load**: < 3 seconds to interactive
- **Animation Frame Rate**: 30fps minimum (60fps target)
- **Asset Load Time**: < 500ms for next song
- **Memory Usage**: < 150MB on mobile
- **Bundle Size**: < 500KB gzipped (excluding assets)

### Fallback Strategy
```javascript
// Progressive degradation
if (isMobile && isLowEndDevice()) {
  // Disable animations
  // Use static images only
  // Reduce asset quality
  // Disable virtual scrolling (use pagination)
}
```

## AI Asset Generation Tools

### Setup Requirements
- **Stable Diffusion**: ComfyUI or Automatic1111
- **Model**: SDXL 1.0 base
- **ControlNet**: For consistency
- **Hardware**: GPU with 8GB+ VRAM
- **Python**: For MusicBrainz API and batch processing

### Generation Pipeline
```python
# High-level workflow
def generate_label_for_song(song_metadata):
    # 1. Query MusicBrainz for label info
    label_data = musicbrainz.get_label_info(song_metadata.mbid)
    
    # 2. Determine label design template
    template = map_label_to_template(label_data.label_name)
    
    # 3. Generate base label with Stable Diffusion
    base_image = sd_generate(
        prompt=build_prompt(song_metadata, template),
        reference_images=load_label_references(template),
        resolution=2048
    )
    
    # 4. Add crisp text overlay (PIL)
    final_image = add_text_overlay(base_image, song_metadata)
    
    # 5. Save to assets/records/labels/{song_id}.png
    save_image(final_image, song_metadata.song_id)
```

### Reference Collection
- Web search for authentic 1970s label designs
- Sources: Discogs, 45cat, RateYourMusic
- Collect 3-5 reference images per major label
- Extract color palettes for consistency

## Implementation Phases

### Phase 1: Core UI (Week 1-2)
- React component structure
- SSE integration with existing backend
- Full jukebox view with now-playing display
- Basic click-to-zoom on selection panel

### Phase 2: Asset Pipeline (Week 2-3)
- Set up Stable Diffusion environment
- Generate first 100 labels for testing
- Create turntable static assets
- Implement asset loading/caching system

### Phase 3: Selection Interface (Week 3-4)
- Build 10x10 button grid with virtual scrolling
- Implement search/filter
- Thumbnail generation and lazy loading
- Mobile touch optimization

### Phase 4: Animation System (Week 5-6)
- Canvas-based animation engine
- Real-time asset composition
- Preloading system (30s before transition)
- Fallback mechanisms

### Phase 5: Mobile Optimization (Week 7-8)
- Responsive layouts
- Performance profiling and optimization
- Progressive enhancement
- Cross-device testing

### Phase 6: Production (Week 9)
- Background label generation for full 10k library
- Error handling and monitoring
- Documentation
- Deployment

## Key Decisions

✅ **Communication**: Initial HTTP GET + SSE for updates  
✅ **Library**: 10,000 songs, progressive asset generation  
✅ **Animation**: Hybrid (pre-rendered static + dynamic composition)  
✅ **Style**: 1970s authentic record labels via MusicBrainz data  
✅ **Mobile**: Responsive assets, virtual scrolling, aggressive caching  

## Integration Checklist

- [ ] Confirm SSE endpoint format with existing backend
- [ ] Verify MusicBrainz data availability in song metadata
- [ ] Test POST /enqueue integration
- [ ] Extract jukebox image region coordinates
- [ ] Set up Stable Diffusion environment
- [ ] Generate test labels for 10 songs
- [ ] Measure mobile performance on target devices
- [ ] Define asset CDN/serving strategy
