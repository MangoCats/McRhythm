# Psychedelic Festival UI - High Level Specification

## Overview
Immersive late 1960s psychedelic rock festival interface. Think: Woodstock, Monterey Pop, liquid light shows, concert posters, peace symbols, and kaleidoscopic visuals. Features organic flowing animations, hand-drawn aesthetics, and tie-dye color explosions.

## Visual Theme References

### Design Inspiration
- **Festival Elements**: Stage, crowds, camping, VW buses, mud, flowers
- **Art Movement**: Concert posters (Wes Wilson, Victor Moscoso, Rick Griffin)
- **Light Shows**: Liquid projections, oil and water, color organs
- **Graphics**: Paisley patterns, mandalas, op-art, art nouveau curves
- **Culture**: Flower power, peace signs, Eastern mysticism, album cover art
- **Colors**: Vibrant, saturated, clashing hues that shouldn't work but do

## Integration Points

### Backend Communication
```rust
// Same as previous UIs - reuse existing backend
GET /festival/state
GET /festival/events (SSE)
POST /festival/enqueue

// Optional mood/energy tracking
struct SongMetadata {
    // Standard fields
    song_id: String,
    title: String,
    artist: String,
    album: Option<String>,
    
    // Extended for psychedelic aesthetic
    album_art_url: Option<String>,    // Original album cover
    genre_tags: Vec<String>,           // "psychedelic rock", "folk", "blues"
    year: Option<u16>,                 // For period-accurate styling
    mood: Option<String>,              // "mellow", "energetic", "cosmic"
}
```

## UI Concept: The Festival Stage

### Main View - Stage & Crowd
```
┌─────────────────────────────────────────────────┐
│  [LIQUID LIGHT SHOW PROJECTION - FLOWING COLORS]│
│  [Swirling, morphing patterns behind stage]     │
│                                                  │
│    ┌────────────────────────────────────┐       │
│    │   ╔═══════════════════════════╗    │       │
│    │   ║    NOW ON STAGE:          ║    │       │
│    │   ║    [Hand-lettered style]  ║    │       │
│    │   ║    ARTIST NAME            ║    │       │
│    │   ║    "Song Title"           ║    │       │
│    │   ╚═══════════════════════════╝    │       │
│    └────────────────────────────────────┘       │
│    [Stage platform with amplifiers]             │
│                                                  │
│  [ANIMATED CROWD - Swaying, hands raised]       │
│  [Smoke/incense drifting across screen]         │
│  [Flowers, peace signs floating up]             │
└─────────────────────────────────────────────────┘
```

### Album/Poster Selection View
```
┌─────────────────────────────────────────────────┐
│  ╔══════════ CONCERT POSTER WALL ═════════════╗ │
│  ║                                             ║ │
│  ║  [Grid of psychedelic concert posters]     ║ │
│  ║  Each poster = album cover or generated    ║ │
│  ║  psychedelic art for the album             ║ │
│  ║                                             ║ │
│  ║  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐          ║ │
│  ║  │POST1│ │POST2│ │POST3│ │POST4│          ║ │
│  ║  │ ART │ │ ART │ │ ART │ │ ART │          ║ │
│  ║  └─────┘ └─────┘ └─────┘ └─────┘          ║ │
│  ║                                             ║ │
│  ║  [Warped text, swirling letters]           ║ │
│  ║  [Each poster morphs/breathes slightly]    ║ │
│  ╚═════════════════════════════════════════════╝ │
│                                                  │
│  Alternative: Vinyl Bins                         │
│  [Like jukebox but with psychedelic labels]     │
└─────────────────────────────────────────────────┘
```

### Expanded View - Cosmic Trip
```
┌─────────────────────────────────────────────────┐
│                                                  │
│  [FULLSCREEN LIQUID LIGHT SHOW]                 │
│  - Morphing fractal patterns                    │
│  - Color-shifting mandalas                      │
│  - Kaleidoscope effects                         │
│  - Oil-on-water simulation                      │
│  - Reactive to music frequencies                │
│                                                  │
│  [Minimal UI overlay]                           │
│  [Exit: Tap anywhere or wait for song end]      │
│                                                  │
└─────────────────────────────────────────────────┘
```

## Asset Generation Strategy

### Visual Elements (AI Generated)

#### 1. Concert Poster Style Album Art
**Generation Approach**:
```python
poster_prompt = """
1960s psychedelic concert poster, {artist_name} performing,
Fillmore Auditorium style, art nouveau influences,
swirling hand-lettered text, vibrant clashing colors,
{colors: orange, purple, green, pink},
ornate decorative borders, mushrooms, flowers, paisley patterns,
vintage lithograph printing aesthetic, slight paper texture,
artist: {artist}, venue: [Cosmic Festival], date: {year}

Style references: Wes Wilson, Victor Moscoso, Rick Griffin
"""

# If album art exists, transform it
album_transform_prompt = """
Transform this album cover into 1960s psychedelic concert poster style,
add swirling decorative borders, enhance colors to vibrant saturation,
add art nouveau flourishes, hand-lettered text treatment,
maintain original subject but amplify psychedelic aesthetic
"""
```

**Poster Elements**:
- Artist name in impossibly warped lettering
- Song/album title integrated into design
- Decorative borders: flowers, mushrooms, peace symbols
- Vibrant color combinations: purple/orange, pink/green, yellow/blue
- Ornate frames and flourishes
- Faux printing artifacts (halftone dots, color registration errors)

#### 2. Liquid Light Show Effects
**Real-time Generation** (Shader-based):
```glsl
// Flowing, organic patterns
// Inspired by overhead projectors with oil/water
```

**Characteristics**:
- Slow, hypnotic motion
- Colors blend and separate
- Organic, blob-like shapes
- Never repeating (procedural)
- Reactive to bass frequencies (pulsing)

**Implementation**:
```javascript
// WebGL shader for liquid light show
class LiquidLightShow {
  constructor() {
    this.colors = [
      [1.0, 0.4, 0.0],  // Orange
      [0.8, 0.0, 0.8],  // Purple
      [0.0, 1.0, 0.5],  // Green
      [1.0, 0.8, 0.0],  // Yellow
    ];
    this.time = 0;
  }
  
  render(audioData) {
    // Perlin noise-based fluid simulation
    // Mix colors based on noise fields
    // Warp UVs with sine waves
    // Add chromatic aberration
  }
}
```

#### 3. Stage Scene Elements
**Festival Stage**:
- Hand-drawn illustration style
- Marshall amp stacks
- Microphone stands
- Tangled cables
- Stage lights (simple circles with rays)
- Wooden platform texture

**Crowd**:
- Simplified silhouettes
- Long hair, bell bottoms
- Peace signs, raised hands
- Swaying motion (sine wave)
- 3-4 depth layers for parallax

**Floating Elements**:
- Flowers drifting up from crowd
- Peace symbols rotating slowly
- Butterflies fluttering
- Incense smoke trails
- Bubbles catching light

### Generation Strategy

```yaml
assets_needed:
  concert_posters: 10000  # One per song/album
  stage_background: 1     # Reusable
  crowd_layers: 4         # Parallax depths
  floating_elements: 20   # Flowers, symbols, etc.
  
generation_priority:
  immediate:
    - Stage and crowd illustrations
    - Liquid light show shader
    - 50 test posters
  
  progressive:
    - Tier 1: Top 100 albums (high detail posters)
    - Tier 2: Top 1000 albums (medium detail)
    - Tier 3: Full catalog (simplified generation)
  
techniques:
  posters_with_album_art:
    method: "Img2Img transformation"
    steps: 25
    strength: 0.6  # Keep original visible
    time: 20s per poster
  
  posters_without_album_art:
    method: "Text-to-Image generation"
    steps: 30
    time: 30s per poster
  
  fallback:
    method: "Template-based with generative flourishes"
    time: 5s per poster
```

## Animation Styles

### 1. Liquid Light Show (Background)
**Technique**: WebGL shader with Perlin noise
```glsl
// Pseudocode
float noise1 = perlin(uv * 2.0 + time * 0.1);
float noise2 = perlin(uv * 3.0 - time * 0.15);
vec2 warp = vec2(noise1, noise2) * 0.3;

vec2 uv_warped = uv + warp;
vec3 color = mix(color1, color2, noise1);
color = mix(color, color3, noise2);

// Add chromatic aberration for trippy effect
vec3 final = vec3(
  texture(uv_warped + 0.01).r,
  texture(uv_warped).g,
  texture(uv_warped - 0.01).b
);
```

**Parameters**:
- Scroll speed: 0.05 - 0.2 (slow, hypnotic)
- Color rotation: Cycle through palette
- Distortion amount: Respond to bass (0.2 - 0.5)
- Blur: Slight (for dreamy effect)

### 2. Poster Breathing
**Technique**: CSS/Canvas scale animation
```css
@keyframes breathe {
  0%, 100% { transform: scale(1.0); }
  50% { transform: scale(1.03); }
}

.poster {
  animation: breathe 8s ease-in-out infinite;
  animation-delay: calc(var(--index) * 0.5s); /* Stagger */
}
```

### 3. Crowd Sway
**Technique**: Sine wave displacement
```javascript
function animateCrowd(time) {
  crowds.forEach((layer, index) => {
    const sway = Math.sin(time * 0.001 + index) * 5;
    layer.style.transform = `translateX(${sway}px)`;
  });
}
```

### 4. Floating Elements
**Technique**: Particle system
```javascript
class FloatingParticle {
  constructor(type) {
    this.type = type;  // 'flower', 'peace', 'butterfly'
    this.x = Math.random() * width;
    this.y = height + 50;
    this.speed = 0.5 + Math.random() * 1.0;
    this.wobble = Math.random() * Math.PI * 2;
  }
  
  update(delta) {
    this.y -= this.speed * delta;
    this.wobble += delta * 0.002;
    this.x += Math.sin(this.wobble) * 0.5;
    
    // Rotate slowly
    this.rotation += delta * 0.001;
    
    // Fade out at top
    if (this.y < 100) {
      this.opacity = this.y / 100;
    }
  }
}
```

### 5. Track Transition - "Melting" Effect
```javascript
// Posters melt/morph into each other
function transitionTracks(fromPoster, toPoster) {
  // Liquid dissolve effect
  // - Current poster "melts" downward
  // - Colors run and blend
  // - New poster emerges from color pool
  // - Solidifies back into shape
  // Duration: 3 seconds
  
  applyShader('liquidDissolve', {
    from: fromPoster,
    to: toPoster,
    progress: animated(0 to 1, 3000ms)
  });
}
```

## UI Components

### View States

#### 1. Main Festival View (Default)
**Layout**:
- Top 40%: Liquid light show background
- Middle 20%: Stage with now-playing display
- Bottom 40%: Animated crowd

**Interactive Elements**:
- Tap stage → song info details
- Tap background → fullscreen light show
- Swipe up → poster selection view
- Swipe left/right → skip tracks

**Visual Details**:
- Hand-drawn stage illustration
- Warped psychedelic text for artist/title
- Flowers and peace signs float upward
- Smoke/incense drifts across
- Crowd sways to beat

#### 2. Poster Wall View
**Layout**:
- Grid of concert poster art
- 4 columns on desktop, 2 on mobile
- Each poster unique to album/song
- Virtual scroll for thousands

**Effects**:
- Posters "breathe" (subtle scale)
- Hover: Rotate slightly, glow
- Colors shift slowly (hue rotation)
- Click: Zooms in, adds to queue

**Typography**:
- Search bar: Hand-lettered style
- Filters: Groovy button designs
- "Far out!", "Groovy!", "Heavy!" feedback

#### 3. Cosmic Trip Mode (Fullscreen)
**Layout**:
- Edge-to-edge liquid light show
- Minimal UI (just song info)
- Progress bar: Flowing liquid

**Features**:
- Kaleidoscope effects
- Mandala patterns
- Fractal zoom
- Color cycling
- Audio-reactive intensity

### Component Architecture (React)
```
src/
  components/
    FestivalUI.jsx           # Main container
    LiquidLightShow.jsx      # WebGL background
    FestivalStage.jsx        # Stage illustration
    CrowdAnimation.jsx       # Swaying crowd
    PosterWall.jsx           # Album selection
    FloatingParticles.jsx    # Flowers, peace signs
    PsychedelicText.jsx      # Warped typography
    MeltTransition.jsx       # Track change effect
  
  effects/
    liquidShader.js          # WebGL liquid simulation
    kaleidoscope.js          # Mandala generator
    colorCycler.js           # Hue rotation
    warpText.js              # Text distortion
  
  utils/
    posterGenerator.js       # AI generation wrapper
    albumArtLoader.js        # Fetch original art
    particleSystem.js        # Floating elements
```

## Mobile Performance Strategies

### Critical Optimizations

1. **Adaptive Liquid Light Quality**
   ```javascript
   const liquidQuality = {
     mobile_low: {
       resolution: 256,      // Very low res, upscaled
       fps: 15,
       effects: ['simple_gradient'],
       blur: 'css',          // Hardware accelerated
     },
     
     mobile_high: {
       resolution: 512,
       fps: 30,
       effects: ['perlin_noise', 'color_mix'],
       blur: 'canvas',
     },
     
     desktop: {
       resolution: 1024,
       fps: 60,
       effects: ['perlin_noise', 'color_mix', 'chromatic_aberration', 'feedback'],
       blur: 'webgl',
     }
   };
   ```

2. **Poster Preloading Strategy**
   ```javascript
   // Load posters progressively
   const preloadStrategy = {
     immediate: [
       'current_playing_poster',
       'next_3_in_queue',
     ],
     
     lazy: [
       'visible_in_viewport',  // IntersectionObserver
       'adjacent_to_viewport',  // +/- 1 screen
     ],
     
     background: [
       'most_played',
       'recently_played',
     ]
   };
   
   // Use placeholder while loading
   const placeholder = generateGradient(song.artist);
   ```

3. **Simplified Effects on Mobile**
   ```javascript
   if (isMobile) {
     // Use CSS gradients instead of WebGL
     background: `
       linear-gradient(45deg, #ff00ff, #00ffff),
       linear-gradient(-45deg, #ffff00, #ff00ff)
     `;
     animation: gradientShift 10s ease infinite;
   } else {
     // Full WebGL liquid simulation
     renderLiquidLightShow();
   }
   ```

4. **Particle System Optimization**
   ```javascript
   const particleCount = {
     mobile_low: 10,
     mobile_high: 30,
     desktop: 100,
   };
   
   // Use CSS animations for mobile
   .floating-flower {
     animation: float 5s ease-in-out infinite;
   }
   
   // Canvas particles only on desktop
   if (!isMobile) {
     renderParticleSystem(canvas);
   }
   ```

5. **Texture Atlases**
   ```javascript
   // Combine all flower/peace sign sprites
   const atlas = createTextureAtlas([
     'flower_1', 'flower_2', 'flower_3',
     'peace_sign', 'butterfly_1', 'butterfly_2'
   ]);
   
   // Single draw call for all particles
   drawBatch(atlas, particles);
   ```

6. **Smart Shader Compilation**
   ```javascript
   // Compile shaders only when needed
   let liquidShader = null;
   
   function enableCosmicMode() {
     if (!liquidShader) {
       // Lazy load and compile
       liquidShader = compileShader(liquidLightVertex, liquidLightFragment);
     }
     liquidShader.use();
   }
   ```

7. **CSS Hardware Acceleration**
   ```css
   .liquid-background {
     will-change: transform, filter;
     transform: translate3d(0, 0, 0);
   }
   
   .poster {
     backface-visibility: hidden;
     transform: translateZ(0);
   }
   ```

8. **Reduced Motion Support**
   ```javascript
   const prefersReducedMotion = 
     window.matchMedia('(prefers-reduced-motion: reduce)').matches;
   
   if (prefersReducedMotion) {
     // Static gradient background
     // No floating particles
     // No breathing animation
     // Simple fade transitions
   }
   ```

9. **Progressive JPEG Loading**
   ```javascript
   // Load posters in stages
   async function loadPoster(songId) {
     // 1. Show color palette immediately
     showPalettePlaceholder(songId);
     
     // 2. Load thumbnail (64x64)
     const thumb = await loadImage(`${songId}_thumb.jpg`);
     showBlurred(thumb);
     
     // 3. Load full resolution
     const full = await loadImage(`${songId}_full.jpg`);
     showCrisp(full);
   }
   ```

10. **Memory-Efficient Caching**
    ```javascript
    // LRU cache with aggressive eviction
    const posterCache = new Map();
    const maxCacheSize = isMobile ? 20 : 100;
    
    function cacheManagement() {
      if (posterCache.size > maxCacheSize) {
        // Evict oldest non-visible
        const toRemove = Array.from(posterCache.keys())
          .filter(id => !isInQueue(id) && !isVisible(id))
          .slice(0, 10);
        
        toRemove.forEach(id => {
          posterCache.delete(id);
          URL.revokeObjectURL(id);  // Free memory
        });
      }
    }
    ```

## Performance Budgets

### Target Metrics
- **Initial Load**: < 2.5 seconds to first visual
- **Liquid Animation**: 30fps minimum (60fps desktop)
- **Poster Load**: < 300ms for visible posters
- **Transition**: < 2 seconds melt effect
- **Memory**: < 120MB on mobile
- **Bundle Size**: < 450KB gzipped

### Visual Complexity Budget
```javascript
const effectBudget = {
  mobile_low: {
    background: 'css_gradient',
    particles: 0,
    posters: 'static',
    crowd: 'static_image',
    quality: 'low'
  },
  
  mobile_high: {
    background: 'canvas_2d',
    particles: 30,
    posters: 'breathing',
    crowd: 'simple_sway',
    quality: 'medium'
  },
  
  desktop: {
    background: 'webgl_liquid',
    particles: 100,
    posters: 'breathing_hue_shift',
    crowd: 'full_parallax',
    effects: ['kaleidoscope', 'chromatic_aberration'],
    quality: 'high'
  }
};
```

## Asset Generation Details

### Psychedelic Poster Art

#### Style Characteristics
```yaml
poster_styles:
  wes_wilson:
    description: "Impossible to read lettering, Art Nouveau curves"
    colors: ["#FF6B35", "#4ECDC4", "#F7B731"]
    features:
      - "Letterforms fill entire space"
      - "Organic flowing shapes"
      - "High contrast"
      - "Ornate borders"
  
  victor_moscoso:
    description: "Vibrating color contrasts, op-art influence"
    colors: ["#FF00FF", "#00FF00", "#FF0000", "#0000FF"]
    features:
      - "Clashing complementary colors"
      - "Visual vibration effects"
      - "Warped perspective"
      - "Comic book influence"
  
  rick_griffin:
    description: "Mystical imagery, eyeballs, flying eyeballs"
    colors: ["#8B00FF", "#FF1493", "#00CED1"]
    features:
      - "Winged eyeballs"
      - "Egyptian/mystical symbols"
      - "Detailed line work"
      - "Psychedelic creatures"
  
  generic_60s:
    description: "Catch-all for period style"
    colors: ["#FFB900", "#FF6B9D", "#00D9FF"]
    features:
      - "Paisley patterns"
      - "Mushrooms and flowers"
      - "Peace symbols"
      - "Mandalas"
```

#### Generation Pipeline
```python
def generate_psychedelic_poster(song_metadata):
    """Generate concert poster for song/album"""
    
    # Determine style based on genre/year
    if song_metadata.genre == "psychedelic_rock":
        style = random.choice(['wes_wilson', 'victor_moscoso'])
    elif song_metadata.genre == "folk":
        style = 'rick_griffin'
    else:
        style = 'generic_60s'
    
    # Get style template
    template = POSTER_STYLES[style]
    
    # Base generation
    if song_metadata.album_art_url:
        # Transform existing album art
        base = img2img_transform(
            image=download_album_art(song_metadata.album_art_url),
            prompt=build_psychedelic_prompt(song_metadata, template),
            strength=0.6,
            style=template
        )
    else:
        # Generate from scratch
        base = txt2img_generate(
            prompt=build_psychedelic_prompt(song_metadata, template),
            style=template
        )
    
    # Add warped text
    poster = add_psychedelic_text(
        base_image=base,
        artist=song_metadata.artist,
        title=song_metadata.title,
        style=template
    )
    
    # Add decorative elements
    poster = add_border_flourishes(poster, template)
    
    # Add vintage printing artifacts
    poster = add_printing_artifacts(poster)
    
    return poster
```

#### Text Warping Algorithm
```python
def warp_psychedelic_text(text, style='flowing'):
    """Create impossibly warped lettering"""
    
    # Start with rendered text
    img = render_text(text, font='Cooper Black', size=200)
    
    # Apply distortions
    if style == 'flowing':
        # Sine wave along reading direction
        img = wave_distort(img, frequency=3, amplitude=30)
        
    elif style == 'impossible':
        # Escher-like impossible geometry
        img = perspective_warp(img, vanishing_points=3)
        
    elif style == 'melting':
        # Dripping letters
        img = liquid_distort(img, gravity=0.5)
    
    # Add outlines and fills
    img = add_outline(img, color=complementary_color, width=5)
    
    return img
```

### Liquid Light Show Shader

```glsl
// Vertex shader
varying vec2 vUv;

void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}

// Fragment shader
uniform float time;
uniform vec3 color1;
uniform vec3 color2;
uniform vec3 color3;
uniform float audioIntensity;

varying vec2 vUv;

// Perlin noise function (simplified)
float noise(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    vec2 uv = vUv;
    
    // Create flowing motion
    float n1 = noise(uv * 2.0 + time * 0.05);
    float n2 = noise(uv * 3.0 - time * 0.08);
    float n3 = noise(uv * 1.5 + time * 0.03);
    
    // Warp the UV coordinates
    vec2 warp = vec2(
        n1 - 0.5,
        n2 - 0.5
    ) * 0.4;
    
    uv += warp;
    
    // Mix colors based on noise
    vec3 color = mix(color1, color2, n1);
    color = mix(color, color3, n2);
    
    // Add pulsing based on audio
    color *= (1.0 + audioIntensity * 0.3);
    
    // Add slight chromatic aberration for trippy effect
    float aberration = 0.01 * audioIntensity;
    vec3 finalColor = vec3(
        mix(color1, color2, noise(uv + aberration)).r,
        color.g,
        mix(color2, color3, noise(uv - aberration)).b
    );
    
    gl_FragColor = vec4(finalColor, 1.0);
}
```

### Festival Scene Illustration

```yaml
illustration_elements:
  stage:
    method: "Hand-drawn style via AI"
    prompt: |
      1960s outdoor concert stage, hand-drawn illustration,
      woodstock style, amplifiers stacked, microphone stands,
      psychedelic paint on equipment, tangled cables,
      wooden platform, warm colors, vintage poster art style
    style: "Flat colors, bold outlines, slight texture"
  
  crowd:
    layers: 4  # For parallax
    prompt: |
      Crowd silhouettes at outdoor concert, 1960s hippies,
      long hair, peace signs raised, swaying, bell bottoms,
      hand-drawn illustration style, simple shapes,
      layer {depth}: {foreground/midground/background/distant}
  
  amplifiers:
    prompt: |
      Marshall amplifier stacks, 1960s concert equipment,
      hand-drawn illustration, front view, logo visible,
      warm colors, slight distortion, vintage poster style
```

## Implementation Phases

### Phase 1: Core UI & Illustrations (Week 1-2)
- React component structure
- SSE integration (reuse from previous UIs)
- Draw/generate stage illustration
- Create crowd layer illustrations
- Static poster wall layout
- Basic CSS gradient background

### Phase 2: Liquid Light Show (Week 2-3)
- WebGL shader development
- Perlin noise implementation
- Color cycling system
- Audio reactivity
- Mobile fallback (CSS gradients)

### Phase 3: Poster Generation (Week 3-5)
- Set up Stable Diffusion
- Create style templates (Wes Wilson, Moscoso, Griffin)
- Text warping algorithm
- Batch generate 100 test posters
- Progressive generation pipeline

### Phase 4: Animations (Week 4-5)
- Floating particle system
- Poster breathing effect
- Crowd sway animation
- Melt transition between tracks
- Touch gestures

### Phase 5: Poster Wall Interface (Week 5-6)
- Virtual scrolling grid
- Search with psychedelic styling
- Genre filters
- Hover effects
- Mobile touch optimization

### Phase 6: Full Catalog Generation (Week 6-8)
- Generate posters for 10k library
- Priority-based generation
- Quality control
- Fallback template system

### Phase 7: Mobile Optimization (Week 7-8)
- Device tier detection
- Simplified shaders
- Reduced particle counts
- Memory management
- Performance profiling

### Phase 8: Polish & Production (Week 9)
- Cosmic trip fullscreen mode
- Kaleidoscope effects
- Final visual polish
- Error handling
- Documentation

## Technical Stack

### Frontend
```javascript
{
  "react": "^18",
  "three": "^0.160",           // WebGL
  "simplex-noise": "^4.0",     // Perlin noise for liquid effect
  "framer-motion": "^10",      // Smooth animations
  "react-spring": "^9",        // Physics-based animations
  "canvas-confetti": "^1.6"    // Particle effects
}
```

### AI Generation
- **Stable Diffusion SDXL**: Poster generation
- **Img2Img**: Transform existing album art
- **ControlNet**: Maintain composition consistency
- **Style LoRAs**: Train on Wes Wilson, Moscoso, Griffin posters if needed

### Shaders & Effects
- **WebGL/Three.js**: Liquid light show (desktop)
- **Canvas 2D**: Simplified effects (mobile)
- **CSS Gradients**: Fallback background
- **SVG Filters**: Text warping effects

## Key Design Decisions

✅ **Theme**: Late 1960s psychedelic rock festival (Woodstock era)  
✅ **Art Style**: Concert poster art (Wes Wilson, Victor Moscoso)  
✅ **Background**: Liquid light show (WebGL shader)  
✅ **Selection**: Poster wall (album art as concert posters)  
✅ **Animation**: Melting transitions, floating flowers, breathing posters  
✅ **Colors**: Vibrant, clashing, saturated (purple/orange, pink/green)  
✅ **Typography**: Warped, impossible-to-read lettering  
✅ **Mobile**: Aggressive optimization, CSS fallbacks  

## Comparison with Other UIs

| Aspect | Jukebox (1970s) | Dance Club (1980s) | Festival (1960s) |
|--------|-----------------|-------------------|------------------|
| **Theme** | Mechanical diner | Digital nightclub | Outdoor concert |
| **Art Style** | Realistic labels | Neon geometric | Hand-drawn posters |
| **Selection** | Button grid | Crate digging | Poster wall |
| **Animation** | Mechanical | Electronic/digital | Organic/flowing |
| **Effects** | Minimal | Strobes/lasers | Liquid lights |
| **Colors** | Warm/chrome | Neon on black | Vibrant clashing |
| **Typography** | Clean/period | LED/terminal | Warped/impossible |
| **Motion** | Precise | Beat-synced | Flowing/hypnotic |
| **Tech** | Mechanical | Early digital | Analog projections |

## Integration Checklist

- [ ] Reuse backend SSE infrastructure
- [ ] Set up Stable Diffusion SDXL
- [ ] Collect concert poster references (Wes Wilson, Moscoso, Griffin)
- [ ] Test img2img transformation on album art
- [ ] Prototype liquid light shader
- [ ] Draw stage illustration
- [ ] Generate crowd layer silhouettes
- [ ] Create 10 test posters (mix of styles)
- [ ] Test WebGL performance on mobile
- [ ] Build text warping algorithm
- [ ] Profile memory usage with particle system
- [ ] Test with real album art URLs from MusicBrainz

## Easter Eggs & Details

**Hidden Interactions**:
- Triple-tap background → Full screen cosmic trip
- Shake device → Trigger "far out!" particle burst
- Long-press poster → Show original album art

**Period-Accurate Details**:
- Stage announcements in hand-lettered style
- "Technical difficulties" messages with peace signs
- Loading states: "Tuning in..." "Getting groovy..."
- Error messages: "Bummer, man" "Bad trip"

**Audio-Reactive Elements**:
- Liquid colors pulse with bass
- Crowd jumps on beat
- Flowers burst on significant beats
- Stage lights flicker with treble
