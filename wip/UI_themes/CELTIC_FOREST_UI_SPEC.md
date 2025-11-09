# Celtic Forest UI - High Level Specification

## Overview
Immersive Celtic forest and Renaissance Faire themed interface with dynamic time-of-day atmosphere. Features medieval castle grounds, ancient trees, parchment scrolls, and realistic astronomy (sun position, moon phases). Transitions from dappled forest sunlight at noon to moonlit lakeside at night.

## Visual Theme References

### Design Inspiration
- **Settings**: Forest glade, castle courtyard, lakeside pavilion
- **Architecture**: Stone walls, timber frames, torch sconces, banners
- **Natural Elements**: Ancient oaks, ivy, moss, wildflowers, lake reflections
- **Artifacts**: Parchment scrolls, illuminated manuscripts, Celtic knotwork
- **Time Dynamics**: Realistic sun/moon positions, seasonal colors
- **Culture**: Minstrels, bards, troubadours, medieval musicians
- **Typography**: Uncial script, Gothic blackletter, Celtic calligraphy

## Integration Points

### Backend Communication
```rust
// Same infrastructure with time/location awareness
GET /celtic/state
GET /celtic/events (SSE)
POST /celtic/enqueue

// Extended for atmospheric system
struct CelticMetadata {
    // Standard fields
    song_id: String,
    title: String,
    artist: String,
    album: Option<String>,
    
    // Celtic/Medieval specific
    genre_tags: Vec<String>,        // "celtic", "folk", "medieval", "ambient"
    mood: Option<String>,           // "lively", "peaceful", "mystical", "heroic"
    tempo: Option<String>,          // "dance", "ballad", "march"
    instruments: Vec<String>,       // "harp", "flute", "bodhrán", "bagpipes"
}

struct AtmosphericState {
    user_time: DateTime,            // User's local time
    user_location: Option<(f64, f64)>, // Lat/lon for sun/moon calculation
    sun_position: SolarPosition,    // Azimuth, elevation
    moon_phase: f64,                // 0.0-1.0 (new to full)
    moon_position: LunarPosition,
    weather_mood: String,           // "clear", "misty", "starry"
}
```

## UI Concept: The Enchanted Glade

### Main View - Time-Based Scenes

#### Morning (6am-11am): Forest Dawn
```
┌─────────────────────────────────────────────────┐
│  [Golden sunrise filtering through trees]       │
│  [Morning mist rolling across ground]           │
│  [Birds flying, deer in distance]               │
│                                                  │
│     🌳    🌲         🌳                         │
│                                                  │
│        ┌─────────────────────┐                 │
│        │  ═══ Now Playing ═══ │  [Parchment    │
│        │  [Unfurled scroll]   │   scroll with  │
│        │  Artist: [Celtic]    │   illuminated  │
│        │  Title: [Script]     │   letters]     │
│        │  [Progress vine]     │                 │
│        └─────────────────────┘                 │
│                                                  │
│  [Forest floor with flowers and mushrooms]      │
└─────────────────────────────────────────────────┘
```

#### Noon (11am-2pm): Dappled Sunlight
```
┌─────────────────────────────────────────────────┐
│  [Bright blue sky through canopy]               │
│  [Dappled sunlight shifting on ground]          │
│  [Butterflies, gentle breeze in leaves]         │
│                                                  │
│   🌳        🌲    ☀️         🌳                │
│          [Light rays through leaves]            │
│                                                  │
│        ┌─────────────────────┐                 │
│        │  ═══ Now Playing ═══ │                 │
│        │  [Scroll fully lit]  │                 │
│        │  [Vivid colors]      │                 │
│        └─────────────────────┘                 │
│                                                  │
│  [Vibrant wildflowers, bees visiting]          │
└─────────────────────────────────────────────────┘
```

#### Evening (5pm-8pm): Golden Hour
```
┌─────────────────────────────────────────────────┐
│  [Warm orange/pink sunset sky]                  │
│  [Long shadows across glade]                    │
│  [Fireflies beginning to emerge]                │
│                                                  │
│   🌳    🌲    🌅    🏰                          │
│              [Castle silhouette in distance]    │
│                                                  │
│        ┌─────────────────────┐                 │
│        │  ═══ Now Playing ═══ │                 │
│        │  [Scroll in amber]   │                 │
│        │  [Warm glow]         │                 │
│        └─────────────────────┘                 │
│                                                  │
│  [Torches being lit around clearing]           │
└─────────────────────────────────────────────────┘
```

#### Night (8pm-6am): Moonlit Lake
```
┌─────────────────────────────────────────────────┐
│  [Star-filled sky, Milky Way visible]           │
│  [Moon at correct phase and position]           │
│  [Silhouetted trees against night sky]          │
│                                                  │
│   🌲        🌕         🌲                       │
│          (moon phase)                           │
│                                                  │
│        ┌─────────────────────┐                 │
│        │  ═══ Now Playing ═══ │  [Scroll lit   │
│        │  [Scroll by torch]   │   by nearby    │
│        │  [Flickering light]  │   torch/lamp]  │
│        └─────────────────────┘                 │
│                                                  │
│  [Peaceful lake reflecting moon and stars]      │
│  [Gentle ripples, owl sounds]                   │
└─────────────────────────────────────────────────┘
```

### Song Selection View - The Scriptorium
```
┌─────────────────────────────────────────────────┐
│  ╔═══════════ THE SCRIPTORIUM ══════════════╗  │
│  ║                                           ║  │
│  ║  [Stone chamber with arched ceiling]     ║  │
│  ║  [Candles and oil lamps providing light] ║  │
│  ║                                           ║  │
│  ║  [Grid of scrolls on wooden shelves]     ║  │
│  ║                                           ║  │
│  ║  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐   ║  │
│  ║  │SCROLL│ │SCROLL│ │SCROLL│ │SCROLL│   ║  │
│  ║  │  1   │ │  2   │ │  3   │ │  4   │   ║  │
│  ║  │[Tied]│ │[Tied]│ │[Tied]│ │[Tied]│   ║  │
│  ║  └──────┘ └──────┘ └──────┘ └──────┘   ║  │
│  ║                                           ║  │
│  ║  [Tap scroll → unfurls to show songs]   ║  │
│  ║  [Search: Ornate text input box]        ║  │
│  ║                                           ║  │
│  ╚═══════════════════════════════════════════╝  │
│                                                  │
│  Alternative: Minstrel's Song Book               │
│  [Leather-bound tome with pages turning]         │
└──────────────────────────────────────────────────┘
```

### Queue View - The Herald's Proclamation
```
┌─────────────────────────────────────────────────┐
│  ╔══════════ UPCOMING BALLADS ══════════════╗  │
│  ║                                           ║  │
│  ║  ❖ Currently Playing ❖                   ║  │
│  ║  ┌─────────────────────────────────────┐ ║  │
│  ║  │ [Fully unfurled scroll]             │ ║  │
│  ║  │ Artist: [Illuminated letters]       │ ║  │
│  ║  │ Title: [Celtic calligraphy]         │ ║  │
│  ║  │ [Decorative border, gold leaf]      │ ║  │
│  ║  └─────────────────────────────────────┘ ║  │
│  ║                                           ║  │
│  ║  ❖ Next ❖                                ║  │
│  ║  ┌───────────────────┐                   ║  │
│  ║  │ [Partially rolled] │                  ║  │
│  ║  │ Title visible      │                  ║  │
│  ║  └───────────────────┘                   ║  │
│  ║                                           ║  │
│  ║  ❖ Then ❖                                ║  │
│  ║  [Rolled scroll with wax seal]           ║  │
│  ║                                           ║  │
│  ╚═══════════════════════════════════════════╝  │
└──────────────────────────────────────────────────┘
```

## Asset Generation Strategy

### Visual Elements (AI Generated)

#### 1. Forest Scenes - Time of Day Variants
**Base Scene Components**:
- Ancient oak trees (foreground, midground, background)
- Forest floor (moss, flowers, mushrooms, fallen leaves)
- Castle ruins or tower in distance
- Lake with reflections (night scenes)
- Sky with appropriate lighting

**Generation Strategy**:
```python
# Generate base scene once, create lighting variants
scene_variants = [
    'dawn_6am',      # Golden sunrise, mist
    'morning_9am',   # Bright, clear light
    'noon_12pm',     # Overhead sun, dappled light
    'afternoon_3pm', # Warm side lighting
    'evening_6pm',   # Golden hour, long shadows
    'dusk_8pm',      # Purple/orange sky, fireflies
    'night_10pm',    # Deep blue, moonlight
    'midnight_12am', # Dark, starlit
]

base_prompt = """
Enchanted Celtic forest glade, ancient oak trees,
mystical atmosphere, {time_of_day} lighting,
high fantasy, detailed vegetation, peaceful clearing,
cinematic lighting, matte painting style, 4k quality
"""

# Time-specific additions
time_prompts = {
    'dawn': 'golden sunrise rays, morning mist, soft pink sky',
    'noon': 'bright overhead sun, dappled shadows, vibrant colors',
    'evening': 'golden hour, warm orange light, long shadows',
    'night': 'moonlight, starry sky, cool blue tones, peaceful'
}
```

#### 2. Moon Phase System
**Real Astronomical Data**:
```python
import ephem

def calculate_moon_phase(datetime, location):
    """Calculate actual moon phase and position"""
    observer = ephem.Observer()
    observer.date = datetime
    observer.lat = str(location.lat)
    observer.lon = str(location.lon)
    
    moon = ephem.Moon(observer)
    
    return {
        'phase': moon.moon_phase,      # 0.0-1.0
        'illumination': moon.phase,     # Percentage
        'altitude': moon.alt,           # Elevation angle
        'azimuth': moon.az,             # Compass direction
        'phase_name': get_phase_name(moon.moon_phase)
    }

def get_phase_name(phase):
    """Convert phase to name"""
    phases = [
        'New Moon', 'Waxing Crescent', 'First Quarter',
        'Waxing Gibbous', 'Full Moon', 'Waning Gibbous',
        'Last Quarter', 'Waning Crescent'
    ]
    index = int(phase * 8) % 8
    return phases[index]
```

**Moon Rendering**:
```yaml
moon_generation:
  method: "AI generate 8 phases + programmatic positioning"
  
  phases_to_generate:
    - new_moon: "Barely visible, thin crescent"
    - waxing_crescent: "Sliver of moon, right side lit"
    - first_quarter: "Half moon, right side"
    - waxing_gibbous: "More than half, approaching full"
    - full_moon: "Complete circle, fully illuminated"
    - waning_gibbous: "More than half, left side lit"
    - last_quarter: "Half moon, left side"
    - waning_crescent: "Thin crescent, left side"
  
  prompt_template: |
    Realistic moon in night sky, {phase_name},
    {illumination}% illuminated, detailed craters,
    soft glow, starfield background, astronomy photography,
    high resolution, accurate lunar features
  
  positioning:
    method: "Calculate real azimuth/altitude"
    place_in_scene: "Use astronomical data for accuracy"
```

#### 3. Parchment Scrolls
**Scroll States**:
- Rolled (tied with ribbon)
- Partially unfurled
- Fully unfurled
- Illuminated (decorative borders)

**Generation Approach**:
```python
scroll_prompt = """
Medieval parchment scroll, aged paper texture,
{state}, soft shadows, realistic lighting,
Celtic knotwork border, illuminated manuscript style,
gold leaf accents, ornate decorative elements,
museum quality, historical artifact photography
"""

# States
scroll_states = {
    'rolled': 'Tightly rolled, tied with red ribbon, wax seal',
    'partial': 'Half unfurled, text partially visible',
    'full': 'Fully open, flat on surface',
    'illuminated': 'Ornate border, gold leaf, decorated capitals'
}

# Text overlay (programmatic)
def add_scroll_text(scroll_image, song_metadata):
    """Add calligraphic text to scroll"""
    # Use Celtic/Gothic fonts
    fonts = [
        'UnifrakturCook',      # Blackletter
        'Caudex',              # Medieval
        'Celtic Garamond',     # Celtic style
    ]
    
    # Illuminated capital letter
    draw_illuminated_capital(scroll_image, song_metadata.title[0])
    
    # Artist/Title in calligraphy
    draw_calligraphy(scroll_image, f"Performed by {song_metadata.artist}")
    draw_calligraphy(scroll_image, song_metadata.title)
    
    # Decorative border
    draw_celtic_border(scroll_image)
    
    return scroll_image
```

#### 4. Celtic Decorative Elements
**Knotwork and Illuminations**:
```yaml
decorative_elements:
  celtic_knots:
    generation: "Procedural or AI"
    styles: ["Trinity knot", "Celtic cross", "Interlaced borders"]
    colors: ["Gold", "Green", "Blue", "Red"]
    usage: "Scroll borders, scene corners, transitions"
  
  illuminated_letters:
    generation: "AI + template"
    prompt: |
      Illuminated manuscript capital letter {letter},
      Celtic Book of Kells style, gold leaf, intricate knotwork,
      vibrant colors, historical medieval art, museum scan
  
  flora_fauna:
    elements: ["Oak leaves", "Ivy vines", "Celtic animals"]
    generation: "AI in medieval illustration style"
    usage: "Scene decoration, scroll embellishments"
```

#### 5. Castle and Architecture
**Background Elements**:
```yaml
architecture:
  castle_tower:
    prompt: |
      Medieval stone castle tower, Celtic/Norman architecture,
      ivy-covered walls, {time_of_day} lighting,
      distant view through forest, atmospheric perspective,
      matte painting, fantasy art
  
  stone_pavilion:
    prompt: |
      Ancient stone pavilion, Celtic pillars, moss-covered,
      forest setting, mystical atmosphere, {lighting}
  
  torch_sconces:
    prompt: |
      Medieval torch in iron sconce, flickering flame,
      mounted on stone wall, {night} lighting,
      realistic fire, warm glow
```

### Dynamic Lighting System

#### Sun Position Calculation
```python
import ephem
from datetime import datetime

class SolarSystem:
    def __init__(self, location):
        self.observer = ephem.Observer()
        self.observer.lat = str(location.lat)
        self.observer.lon = str(location.lon)
    
    def get_sun_position(self, time):
        """Calculate sun position for given time"""
        self.observer.date = time
        sun = ephem.Sun(self.observer)
        
        return {
            'altitude': float(sun.alt) * 180 / ephem.pi,  # Degrees above horizon
            'azimuth': float(sun.az) * 180 / ephem.pi,    # Compass direction
            'is_visible': sun.alt > 0,                     # Above horizon
        }
    
    def get_lighting_state(self, time):
        """Determine which scene to show"""
        sun_pos = self.get_sun_position(time)
        hour = time.hour
        
        if sun_pos['altitude'] < -12:
            return 'night'
        elif sun_pos['altitude'] < 0:
            return 'twilight'
        elif hour < 10:
            return 'morning'
        elif hour < 14:
            return 'noon'
        elif hour < 17:
            return 'afternoon'
        else:
            return 'evening'
```

#### Lighting Transitions
```javascript
class AtmosphericSystem {
  constructor() {
    this.currentScene = null;
    this.transitionDuration = 30000; // 30 seconds
  }
  
  update() {
    const time = new Date();
    const targetScene = this.calculateScene(time);
    
    if (targetScene !== this.currentScene) {
      this.transitionToScene(targetScene);
    }
    
    // Update dynamic elements
    this.updateSunPosition(time);
    this.updateMoonPosition(time);
    this.updateSkyColor(time);
  }
  
  transitionToScene(newScene) {
    // Smooth crossfade between scenes
    anime({
      targets: '.scene-layer',
      opacity: [1, 0],
      duration: this.transitionDuration,
      easing: 'easeInOutQuad',
      complete: () => {
        this.loadScene(newScene);
        anime({
          targets: '.scene-layer',
          opacity: [0, 1],
          duration: this.transitionDuration,
        });
      }
    });
  }
}
```

## Animation Styles

### 1. Scroll Unfurling
```javascript
class ScrollAnimation {
  async unfurl(scroll, duration = 1500) {
    // Start: Rolled up
    scroll.style.transform = 'scaleX(0.1) scaleY(1)';
    scroll.style.transformOrigin = 'left center';
    
    // Unfurl horizontally
    await anime({
      targets: scroll,
      scaleX: [0.1, 1],
      duration: duration,
      easing: 'easeOutElastic(1, 0.8)'
    }).finished;
    
    // Reveal text with fade-in
    const text = scroll.querySelector('.scroll-text');
    await anime({
      targets: text,
      opacity: [0, 1],
      duration: 500,
      easing: 'easeInQuad'
    }).finished;
  }
  
  async roll(scroll, duration = 1000) {
    // Hide text
    const text = scroll.querySelector('.scroll-text');
    await anime({
      targets: text,
      opacity: [1, 0],
      duration: 300
    }).finished;
    
    // Roll up
    await anime({
      targets: scroll,
      scaleX: [1, 0.1],
      duration: duration,
      easing: 'easeInBack(2)'
    }).finished;
  }
}
```

### 2. Track Transition - Scroll Swap
```javascript
async function transitionTracks(fromSong, toSong) {
  // Current scroll rolls up
  await scrollAnimation.roll(currentScroll, 800);
  
  // Next scroll drops from above and unfurls
  nextScroll.style.transform = 'translateY(-100px)';
  nextScroll.style.opacity = 0;
  
  await anime({
    targets: nextScroll,
    translateY: ['-100px', '0px'],
    opacity: [0, 1],
    duration: 600,
    easing: 'easeOutBounce'
  }).finished;
  
  await scrollAnimation.unfurl(nextScroll, 1200);
  
  // Optional: Sparkle effect
  createSparkles(nextScroll.getBoundingClientRect());
}
```

### 3. Environmental Animations

#### Dappled Sunlight (Noon)
```javascript
class DappledLight {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
    this.spots = this.generateLightSpots(20);
  }
  
  generateLightSpots(count) {
    return Array.from({ length: count }, () => ({
      x: Math.random() * this.canvas.width,
      y: Math.random() * this.canvas.height,
      radius: 30 + Math.random() * 50,
      intensity: 0.1 + Math.random() * 0.2,
      speed: 0.001 + Math.random() * 0.002
    }));
  }
  
  animate(time) {
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    
    this.spots.forEach(spot => {
      // Slow drift
      spot.x += Math.sin(time * spot.speed) * 0.5;
      spot.y += Math.cos(time * spot.speed * 0.7) * 0.3;
      
      // Gradual intensity change (leaves moving)
      const pulse = Math.sin(time * spot.speed * 10) * 0.05;
      
      // Draw soft light spot
      const gradient = this.ctx.createRadialGradient(
        spot.x, spot.y, 0,
        spot.x, spot.y, spot.radius
      );
      gradient.addColorStop(0, `rgba(255, 255, 200, ${spot.intensity + pulse})`);
      gradient.addColorStop(1, 'rgba(255, 255, 200, 0)');
      
      this.ctx.fillStyle = gradient;
      this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    });
  }
}
```

#### Water Reflections (Night)
```javascript
class WaterReflection {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
    this.time = 0;
  }
  
  animate() {
    this.time += 0.01;
    
    // Draw base water
    const gradient = this.ctx.createLinearGradient(0, 0, 0, this.canvas.height);
    gradient.addColorStop(0, '#001a33');
    gradient.addColorStop(1, '#000814');
    this.ctx.fillStyle = gradient;
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    
    // Draw moon reflection
    this.drawMoonReflection();
    
    // Add ripples
    this.drawRipples();
  }
  
  drawMoonReflection() {
    const moonX = this.canvas.width * 0.5;
    const moonY = this.canvas.height * 0.3;
    
    // Elongated reflection
    for (let i = 0; i < 50; i++) {
      const y = moonY + i * 5;
      const wobble = Math.sin(y * 0.1 + this.time * 2) * 10;
      const alpha = Math.max(0, 0.3 - i * 0.006);
      
      this.ctx.fillStyle = `rgba(255, 255, 200, ${alpha})`;
      this.ctx.fillRect(moonX + wobble - 20, y, 40, 3);
    }
  }
  
  drawRipples() {
    // Concentric ripples
    for (let i = 0; i < 3; i++) {
      const radius = (this.time * 50 + i * 100) % 300;
      const alpha = Math.max(0, 0.3 - radius / 300);
      
      this.ctx.strokeStyle = `rgba(255, 255, 255, ${alpha})`;
      this.ctx.lineWidth = 2;
      this.ctx.beginPath();
      this.ctx.arc(
        this.canvas.width * 0.5,
        this.canvas.height * 0.3,
        radius,
        0,
        Math.PI * 2
      );
      this.ctx.stroke();
    }
  }
}
```

#### Fireflies (Evening/Night)
```javascript
class Fireflies {
  constructor(count = 30) {
    this.fireflies = Array.from({ length: count }, () => ({
      x: Math.random() * window.innerWidth,
      y: Math.random() * window.innerHeight,
      vx: (Math.random() - 0.5) * 0.5,
      vy: (Math.random() - 0.5) * 0.5,
      brightness: 0,
      phase: Math.random() * Math.PI * 2
    }));
  }
  
  animate(time) {
    this.fireflies.forEach(fly => {
      // Drift motion
      fly.x += fly.vx;
      fly.y += fly.vy;
      
      // Boundary wrap
      if (fly.x < 0) fly.x = window.innerWidth;
      if (fly.x > window.innerWidth) fly.x = 0;
      if (fly.y < 0) fly.y = window.innerHeight;
      if (fly.y > window.innerHeight) fly.y = 0;
      
      // Pulsing brightness
      fly.brightness = (Math.sin(time * 0.005 + fly.phase) + 1) * 0.5;
      
      // Occasional direction change
      if (Math.random() < 0.01) {
        fly.vx = (Math.random() - 0.5) * 0.5;
        fly.vy = (Math.random() - 0.5) * 0.5;
      }
    });
  }
  
  render(ctx) {
    this.fireflies.forEach(fly => {
      const gradient = ctx.createRadialGradient(fly.x, fly.y, 0, fly.x, fly.y, 10);
      gradient.addColorStop(0, `rgba(255, 255, 150, ${fly.brightness})`);
      gradient.addColorStop(1, 'rgba(255, 255, 150, 0)');
      
      ctx.fillStyle = gradient;
      ctx.fillRect(fly.x - 10, fly.y - 10, 20, 20);
    });
  }
}
```

### 4. Progress Indicator - Growing Vine
```javascript
class VineProgress {
  constructor(container) {
    this.svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    this.path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    this.leaves = [];
    
    this.svg.appendChild(this.path);
    container.appendChild(this.svg);
  }
  
  updateProgress(percent) {
    // Generate organic vine path
    const points = this.generateVinePath(percent);
    const pathData = this.pointsToPath(points);
    
    this.path.setAttribute('d', pathData);
    this.path.setAttribute('stroke', '#2d5016');
    this.path.setAttribute('stroke-width', '3');
    this.path.setAttribute('fill', 'none');
    
    // Add leaves at intervals
    this.updateLeaves(points);
  }
  
  generateVinePath(percent) {
    const width = 300;
    const points = [];
    const steps = 50;
    
    for (let i = 0; i <= steps * percent; i++) {
      const x = (i / steps) * width;
      const y = Math.sin(i * 0.3) * 10 + 50; // Gentle wave
      points.push({ x, y });
    }
    
    return points;
  }
  
  pointsToPath(points) {
    if (points.length === 0) return '';
    
    let path = `M ${points[0].x} ${points[0].y}`;
    
    for (let i = 1; i < points.length; i++) {
      const prev = points[i - 1];
      const curr = points[i];
      const cpx = (prev.x + curr.x) / 2;
      const cpy = (prev.y + curr.y) / 2;
      
      path += ` Q ${prev.x} ${prev.y} ${cpx} ${cpy}`;
    }
    
    return path;
  }
  
  updateLeaves(points) {
    // Add leaf every 20 points
    const leafInterval = 20;
    const targetLeafCount = Math.floor(points.length / leafInterval);
    
    while (this.leaves.length < targetLeafCount) {
      const leafPos = this.leaves.length * leafInterval;
      if (leafPos < points.length) {
        const point = points[leafPos];
        this.addLeaf(point.x, point.y);
      }
    }
  }
  
  addLeaf(x, y) {
    const leaf = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    const side = this.leaves.length % 2 === 0 ? 1 : -1;
    
    // Simple leaf shape
    const leafPath = `M ${x} ${y} Q ${x + 5} ${y + side * 8} ${x + 10} ${y} Q ${x + 5} ${y - side * 2} ${x} ${y}`;
    
    leaf.setAttribute('d', leafPath);
    leaf.setAttribute('fill', '#4a7c2f');
    leaf.style.opacity = '0';
    
    this.svg.appendChild(leaf);
    this.leaves.push(leaf);
    
    // Fade in
    anime({
      targets: leaf,
      opacity: [0, 1],
      duration: 500
    });
  }
}
```

## UI Components

### View States

#### 1. Main Forest View (Default)
**Layout**:
- Full-screen background: Time-appropriate forest scene
- Center: Unfurled parchment scroll with now playing
- Bottom: Growing vine progress indicator
- Corners: Celtic knotwork decorations

**Dynamic Elements**:
- Sky color transitions throughout day
- Sun/moon position changes in real-time
- Environmental animations (dappled light, fireflies, etc.)
- Weather effects (light rain, fog) based on mood

#### 2. Queue View - The Herald's Board
**Layout**:
- Stone wall or wooden board background
- Multiple scrolls at different states:
  - Now Playing: Fully unfurled, illuminated
  - Next: Partially rolled, preview visible
  - Queue items: Rolled with wax seals
- Tap scroll to preview or remove from queue

#### 3. Song Selection - The Scriptorium
**Layout**:
- Library interior (stone walls, arched ceiling)
- Shelves of rolled scrolls
- Each scroll category labeled (by genre/mood)
- Search: Ornate text box with Celtic frame
- Candle/torch lighting

**Organization**:
- By Mood: Lively, Peaceful, Mystical, Heroic
- By Instrument: Harp, Flute, Drum, Bagpipes
- By Era: Ancient, Medieval, Renaissance
- Alphabetical: A-Z with illuminated letters

### Component Architecture (React)
```
src/
  components/
    CelticForestUI.jsx         # Main container
    ForestScene.jsx            # Background with time system
    AtmosphericSystem.jsx      # Sun/moon calculations
    ParchmentScroll.jsx        # Scroll display
    ScrollAnimation.jsx        # Unfurl mechanics
    IlluminatedText.jsx        # Calligraphic text
    CelticBorder.jsx           # Decorative knotwork
    VineProgress.jsx           # Progress indicator
    Scriptorium.jsx            # Song selection
    QueueBoard.jsx             # Herald's board
    
    effects/
      DappledLight.jsx         # Noon sunlight
      WaterReflection.jsx      # Night lake
      Fireflies.jsx            # Evening/night
      FallingLeaves.jsx        # Autumn mood
      Butterflies.jsx          # Spring/summer
      MoonPhase.jsx            # Lunar rendering
  
  utils/
    solarCalculations.js       # Sun position
    lunarCalculations.js       # Moon phase/position
    timeOfDay.js               # Scene selection
    celticArt.js               # Knotwork generation
```

## Mobile Performance Strategies

### Critical Optimizations

1. **Simplified Scene Rendering**
   ```javascript
   const sceneQuality = {
     mobile_low: {
       background: 'static_image',  // No parallax
       effects: [],                  // No particles
       transitions: 'crossfade',     // Simple
     },
     
     mobile_high: {
       background: 'static_with_overlay',
       effects: ['fireflies'],       // Limited particles
       transitions: 'crossfade',
       scroll_animation: 'simple',
     },
     
     desktop: {
       background: 'layered_parallax',
       effects: ['dappled_light', 'fireflies', 'water_reflection'],
       transitions: 'full_animation',
       scroll_animation: 'elastic_unfurl',
     }
   };
   ```

2. **Pre-rendered Time Variants**
   ```javascript
   // Instead of calculating lighting in real-time
   const sceneImages = {
     'morning': '/scenes/forest_morning.jpg',
     'noon': '/scenes/forest_noon.jpg',
     'evening': '/scenes/forest_evening.jpg',
     'night': '/scenes/forest_night.jpg'
   };
   
   // Preload next time period
   if (currentTime.hour === 11) {
     preloadImage(sceneImages['noon']);
   }
   ```

3. **Static Moon Position (Mobile)**
   ```javascript
   if (isMobile) {
     // Use pre-rendered moon at fixed position
     moon.src = `/moons/phase_${currentPhase}.png`;
     moon.style.top = '20%';
     moon.style.left = '60%';
   } else {
     // Calculate actual position
     const position = calculateMoonPosition(time, location);
     moon.style.top = position.y;
     moon.style.left = position.x;
   }
   ```

4. **Simplified Scroll Animation**
   ```javascript
   function unfurlScroll(mobile = false) {
     if (mobile) {
       // Simple fade + scale
       anime({
         targets: '.scroll',
         opacity: [0, 1],
         scale: [0.9, 1],
         duration: 300
       });
     } else {
       // Full elastic unfurl
       anime({
         targets: '.scroll',
         scaleX: [0.1, 1],
         duration: 1500,
         easing: 'easeOutElastic(1, 0.8)'
       });
     }
   }
   ```

5. **Reduce Particle Count**
   ```javascript
   const particleCounts = {
     fireflies: isMobile ? 10 : 30,
     butterflies: isMobile ? 5 : 15,
     falling_leaves: isMobile ? 8 : 25,
   };
   ```

6. **CSS-Only Effects (Mobile)**
   ```css
   /* Dappled light using CSS gradients */
   @media (max-width: 768px) {
     .dappled-light {
       background: radial-gradient(
         circle at 30% 40%, 
         rgba(255, 255, 200, 0.2) 0%, 
         transparent 50%
       );
       animation: lightShift 20s ease-in-out infinite;
     }
   }
   
   @keyframes lightShift {
     0%, 100% { opacity: 0.3; }
     50% { opacity: 0.5; }
   }
   ```

7. **Lazy Load Scroll Textures**
   ```javascript
   // Only load scroll backgrounds when needed
   const scrollCache = new Map();
   
   async function getScrollTexture(type) {
     if (!scrollCache.has(type)) {
       const texture = await loadImage(`/scrolls/${type}.png`);
       scrollCache.set(type, texture);
     }
     return scrollCache.get(type);
   }
   ```

8. **Throttle Sun/Moon Updates**
   ```javascript
   // Update astronomical positions less frequently
   const updateInterval = isMobile ? 60000 : 10000; // 1 min vs 10s
   
   setInterval(() => {
     updateSunPosition();
     updateMoonPosition();
   }, updateInterval);
   ```

9. **Simplified Celtic Borders**
   ```javascript
   if (isMobile) {
     // Use simple repeating pattern
     border.style.backgroundImage = 'url(/patterns/celtic_simple.png)';
     border.style.backgroundRepeat = 'repeat-x';
   } else {
     // Render full SVG knotwork
     renderCelticKnotwork(border);
   }
   ```

10. **Progressive Scene Loading**
    ```javascript
    async function loadScene(timeOfDay) {
      // 1. Show blurred placeholder
      showPlaceholder(timeOfDay);
      
      // 2. Load low-res version
      const lowRes = await loadImage(`${timeOfDay}_low.jpg`);
      showImage(lowRes, 'blur(5px)');
      
      // 3. Load full resolution
      const highRes = await loadImage(`${timeOfDay}_high.jpg`);
      transitionTo(highRes);
    }
    ```

## Performance Budgets

### Target Metrics
- **Initial Load**: < 3 seconds to forest view
- **Scene Transition**: < 2 seconds crossfade
- **Scroll Animation**: < 1.5 seconds unfurl
- **Time Update**: < 100ms scene switch
- **Memory**: < 150MB on mobile
- **Bundle Size**: < 500KB gzipped

### Visual Complexity Budget
```javascript
const effectBudget = {
  mobile_low: {
    background: 'static',
    particles: 0,
    scroll: 'fade',
    borders: 'simple',
    progress: 'bar'
  },
  
  mobile_high: {
    background: 'static_layered',
    particles: 10,
    scroll: 'scale_fade',
    borders: 'pattern',
    progress: 'svg_vine'
  },
  
  desktop: {
    background: 'parallax_layered',
    particles: 30,
    scroll: 'elastic_unfurl',
    borders: 'dynamic_svg',
    progress: 'animated_vine',
    astronomical: 'real_time'
  }
};
```

## Asset Generation Details

### Time-of-Day Scene Generation

```yaml
scene_generation:
  base_scene:
    prompt: |
      Enchanted Celtic forest glade, ancient oak trees,
      mystical atmosphere, clearing in center,
      distant castle tower, natural lighting,
      high fantasy, detailed vegetation, peaceful,
      matte painting style, 4k cinematic quality
  
  lighting_variants:
    dawn:
      prompt_addition: "golden sunrise, morning mist, soft pink sky, first light"
      color_temp: 3500K
      ambient: soft_warm
    
    noon:
      prompt_addition: "bright overhead sun, dappled shadows through leaves, vibrant colors"
      color_temp: 5500K
      ambient: bright_neutral
    
    evening:
      prompt_addition: "golden hour, warm orange light, long shadows, fireflies emerging"
      color_temp: 3000K
      ambient: warm_amber
    
    night:
      prompt_addition: "moonlight, starry sky, cool blue tones, peaceful darkness"
      color_temp: 7000K
      ambient: cool_blue
  
  seasonal_variants:
    spring: "fresh green leaves, wildflowers, cherry blossoms"
    summer: "lush foliage, full canopy, vibrant greens"
    autumn: "golden leaves, warm colors, falling foliage"
    winter: "bare branches, light snow, frost, cold atmosphere"
```

### Parchment Scroll Templates

```yaml
scroll_generation:
  base_templates:
    aged_parchment:
      prompt: |
        Medieval parchment scroll, aged paper texture,
        yellowed edges, slight staining, soft lighting,
        museum quality artifact photography, high resolution
    
    illuminated_manuscript:
      prompt: |
        Illuminated manuscript page, gold leaf accents,
        Celtic knotwork border, ornate decorative capitals,
        Book of Kells style, vibrant colors, museum scan
  
  decorative_borders:
    celtic_knotwork:
      elements: ["Trinity knots", "Interlaced patterns", "Celtic spirals"]
      colors: ["Gold (#FFD700)", "Green (#2d5016)", "Blue (#1e3a5f)"]
    
    floral:
      elements: ["Oak leaves", "Ivy vines", "Acorns", "Wildflowers"]
      style: "Medieval botanical illustration"
  
  text_rendering:
    fonts:
      - "UnifrakturCook" # Gothic blackletter
      - "Caudex" # Medieval serif
      - "Cinzel Decorative" # Elegant capitals
    
    illuminated_capitals:
      method: "AI generate per letter"
      prompt: |
        Illuminated manuscript capital letter {letter},
        Celtic Book of Kells style, gold leaf, intricate knotwork,
        vibrant medieval colors, red, blue, green, gold,
        historical medieval art, museum scan quality
```

### Moon Phase Assets

```yaml
moon_generation:
  phases:
    new_moon:
      prompt: "New moon, barely visible, thin crescent, dark sky, stars visible"
      visibility: 0.0
    
    first_quarter:
      prompt: "Half moon, first quarter, right side illuminated, detailed craters"
      visibility: 0.5
    
    full_moon:
      prompt: "Full moon, completely illuminated, lunar maria visible, bright glow"
      visibility: 1.0
  
  rendering:
    resolution: 2048x2048
    style: "Astronomical photography, realistic lunar surface"
    background: "Starfield, Milky Way visible"
    glow: "Soft atmospheric glow around moon"
  
  positioning:
    method: "Astronomical calculation using ephem library"
    update_frequency: "Every 10 seconds (desktop) or 1 minute (mobile)"
```

## Implementation Phases

### Phase 1: Core Scene System (Week 1-2)
- React component structure
- SSE integration (reuse from other UIs)
- Time-of-day detection system
- Generate 4 base scenes (morning/noon/evening/night)
- Static scene display with crossfade transitions

### Phase 2: Astronomical System (Week 2-3)
- Implement sun position calculations
- Implement moon phase calculations
- Generate 8 moon phase images
- Real-time positioning system
- Test with various locations/times

### Phase 3: Parchment Scrolls (Week 3-4)
- Generate scroll templates
- Implement unfurl animation
- Add Celtic calligraphy text rendering
- Create illuminated capital letters
- Test scroll transitions

### Phase 4: Environmental Effects (Week 4-5)
- Dappled light system (noon)
- Water reflections (night)
- Fireflies (evening/night)
- Butterflies (day)
- Falling leaves (autumn)

### Phase 5: Song Selection (Week 5-6)
- Scriptorium interior scene
- Scroll library visualization
- Search and filter interface
- Category organization
- Touch gestures

### Phase 6: Asset Generation (Week 6-7)
- Generate seasonal variants
- Create decorative borders
- Generate illuminated letters (A-Z)
- Batch process scroll backgrounds
- Quality control

### Phase 7: Mobile Optimization (Week 7-8)
- Simplified animations
- Reduced particle counts
- Static scene fallbacks
- Performance profiling
- Battery optimization

### Phase 8: Polish & Production (Week 8-9)
- Celtic border decorations
- Growing vine progress indicator
- Sound effects (optional)
- Error handling
- Documentation

## Technical Stack

### Frontend
```javascript
{
  "react": "^18",
  "anime.js": "^3.2",          // Smooth animations
  "ephem": "^1.0",             // Astronomical calculations (via Python backend)
  "date-fns": "^2.30",         // Time manipulation
  "framer-motion": "^10",      // Page transitions
}
```

### AI Generation
- **Stable Diffusion SDXL**: Scene and scroll generation
- **ControlNet**: Maintain scene composition across lighting variants
- **Img2Img**: Transform base scene for different times
- **Inpainting**: Add/modify scene elements

### Astronomical
- **PyEphem** (Python backend): Calculate sun/moon positions
- **API endpoint**: `/celtic/astronomy?time=...&location=...`

## Key Design Decisions

✅ **Theme**: Celtic forest / Renaissance Faire / Medieval castle  
✅ **Dynamic**: Real-time sun/moon positions based on user time/location  
✅ **Scenes**: 4 time periods (morning/noon/evening/night)  
✅ **Moon**: Accurate phase and position (8 phase variants)  
✅ **Display**: Parchment scrolls with Celtic calligraphy  
✅ **Progress**: Growing vine with leaves  
✅ **Effects**: Dappled light, water reflections, fireflies  
✅ **Mobile**: Simplified scenes, static positions, reduced particles  

## Comparison with Other UIs

| Aspect | Jukebox | Dance Club | Festival | Radio | Celtic |
|--------|---------|------------|----------|-------|--------|
| **Era** | 1970s | 1980s | 1960s | 1970s-80s | Medieval |
| **Setting** | Diner | Nightclub | Concert | Studio | Forest |
| **Format** | 45 singles | 12" vinyl | Album art | Carts | Scrolls |
| **Dynamic** | Static | Beat-synced | Audio-reactive | Real-time | Time-of-day |
| **Mood** | Nostalgic | Energetic | Trippy | Professional | Peaceful |
| **Nature** | None | None | None | None | Central |

## Integration Checklist

- [ ] Reuse backend SSE infrastructure
- [ ] Add astronomy calculation endpoint (sun/moon)
- [ ] Implement time-of-day detection
- [ ] Set up Stable Diffusion for scene generation
- [ ] Generate 4 base time-of-day scenes
- [ ] Generate 8 moon phase images
- [ ] Create parchment scroll templates
- [ ] Design Celtic border patterns
- [ ] Generate illuminated letters (A-Z)
- [ ] Test astronomical calculations accuracy
- [ ] Profile scene transition performance
- [ ] Test with various timezones/locations

## Special Features

### Real Astronomy
```python
# Backend endpoint for accurate calculations
@app.get("/celtic/astronomy")
async def get_astronomy(
    time: datetime,
    latitude: float,
    longitude: float
):
    observer = ephem.Observer()
    observer.date = time
    observer.lat = str(latitude)
    observer.lon = str(longitude)
    
    sun = ephem.Sun(observer)
    moon = ephem.Moon(observer)
    
    return {
        "sun": {
            "altitude": float(sun.alt),
            "azimuth": float(sun.az),
            "is_visible": sun.alt > 0
        },
        "moon": {
            "phase": float(moon.moon_phase),
            "illumination": float(moon.phase),
            "altitude": float(moon.alt),
            "azimuth": float(moon.az),
            "phase_name": get_moon_phase_name(moon.moon_phase)
        },
        "recommended_scene": calculate_scene(sun.alt, time.hour)
    }
```

### Seasonal Variations
```javascript
// Optionally switch scenes by season
const season = getCurrentSeason();

const sceneVariants = {
  spring: 'forest_spring.jpg',   // Flowers, new leaves
  summer: 'forest_summer.jpg',   // Lush, full canopy
  autumn: 'forest_autumn.jpg',   // Golden leaves
  winter: 'forest_winter.jpg'    // Snow, bare branches
};
```

### Sound Design (Optional)
```javascript
const ambientSounds = {
  morning: ['birds_chirping.mp3', 'gentle_breeze.mp3'],
  noon: ['rustling_leaves.mp3', 'distant_stream.mp3'],
  evening: ['crickets.mp3', 'owl_hoot.mp3'],
  night: ['gentle_water.mp3', 'night_insects.mp3']
};

// Very low volume ambient background
playAmbient(ambientSounds[currentTimeOfDay], { volume: 0.1 });
```
