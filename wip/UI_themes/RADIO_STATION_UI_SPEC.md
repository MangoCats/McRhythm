# Radio Station UI - High Level Specification

## Overview
Behind-the-scenes radio station interface inspired by 1970s-80s broadcasting. Think: control room, DJ booth, vinyl carts, reel-to-reel machines, request lines, and on-air signage. Features realistic broadcast equipment aesthetics with retro analog warmth.

## Visual Theme References

### Design Inspiration
- **Broadcast Elements**: Control board, cart machines, turntables, mic booms
- **Studio Design**: Soundproof walls, on-air lights, request sheets, coffee cups
- **Equipment**: Reel-to-reel, cart players, mixing console, meters
- **Atmosphere**: Worn wood paneling, orange/brown tones, incandescent lights
- **Culture**: Phone-in requests, traffic/weather updates, DJ personality
- **Typography**: Helvetica, industrial labels, handwritten request slips

## Integration Points

### Backend Communication
```rust
// Same infrastructure, extended for radio theme
GET /radio/state
GET /radio/events (SSE)
POST /radio/enqueue

// Radio-specific features
struct RadioMetadata {
    // Standard fields
    song_id: String,
    title: String,
    artist: String,
    album: Option<String>,
    
    // Radio-specific
    rotation_status: String,        // "Heavy", "Medium", "Light"
    last_played: Option<DateTime>,  // Avoid repeats
    request_count: u32,             // Track popularity
    intro_duration: Option<f32>,    // Seconds before vocals
    outro_duration: Option<f32>,    // Fade point
    has_explicit_lyrics: bool,      // FCC compliance
}

struct OnAirStatus {
    live: bool,                     // Live broadcast vs automation
    dj_name: Option<String>,        // Current DJ
    show_name: Option<String>,      // "Morning Drive", "Evening Gold"
    mic_active: bool,               // DJ talking over intro
}
```

## UI Concept: The Control Room

### Main View - DJ Booth
```
┌─────────────────────────────────────────────────┐
│  ┌─────────────────────────────────────────┐   │
│  │  🔴 ON AIR         WKRP-FM 102.5       │   │  <- On-air light
│  └─────────────────────────────────────────┘   │
│                                                 │
│  ┌──────────────────────────────────────┐      │
│  │ ◆ NOW PLAYING ◆                      │      │
│  │ ARTIST: [LED-style flip display]     │      │  <- Flip clock style
│  │ TITLE:  [LED-style flip display]     │      │
│  │ TIME:   3:42 / 4:15                  │      │
│  └──────────────────────────────────────┘      │
│                                                 │
│  [VINYL CART MACHINE - 3 carts visible]        │  <- Triple cart player
│  ┌────┐ ┌────┐ ┌────┐                         │
│  │NOW │ │NEXT│ │QUE3│                         │
│  │PLY │ │ UP │ │    │                         │
│  └────┘ └────┘ └────┘                         │
│  [███─] [────] [────]  <- Progress lights     │
│                                                 │
│  [VU METERS - Stereo L/R]                      │
│  L: ▌████████░░░░ 0dB                         │
│  R: ▌█████████░░░ +1dB                        │
│                                                 │
│  [MIXING BOARD - Faders and buttons]           │
└─────────────────────────────────────────────────┘
```

### Request Line View - Phone-In Song Selection
```
┌─────────────────────────────────────────────────┐
│  ╔═══════════ REQUEST LINE ══════════════════╗ │
│  ║                                            ║ │
│  ║  [Stack of handwritten request slips]     ║ │
│  ║                                            ║ │
│  ║  ┌────────────────────────────────┐       ║ │
│  ║  │ From: Mike - 555-1234          │       ║ │
│  ║  │ Request: Journey - Don't Stop  │       ║ │
│  ║  │ Dedicates to: Sarah            │       ║ │
│  ║  │ [ADD TO QUEUE] [CALL BACK]     │       ║ │
│  ║  └────────────────────────────────┘       ║ │
│  ║                                            ║ │
│  ║  OR: Search song library                  ║ │
│  ║  [___________________________]  [SEARCH]  ║ │
│  ║                                            ║ │
│  ╚════════════════════════════════════════════╝ │
│                                                  │
│  Alternative: Music Library - Card Catalog      │
│  [Rows of vinyl cart spines in rack slots]      │
│  Color-coded by genre/rotation status            │
└──────────────────────────────────────────────────┘
```

### Station Manager View - Programming
```
┌─────────────────────────────────────────────────┐
│  ╔════════ PROGRAM DIRECTOR OFFICE ═══════════╗ │
│  ║                                             ║ │
│  ║  [Cork board with index cards]             ║ │
│  ║                                             ║ │
│  ║  HEAVY ROTATION    MEDIUM ROTATION         ║ │
│  ║  ┌──────┐┌──────┐  ┌──────┐┌──────┐       ║ │
│  ║  │Song A││Song B│  │Song E││Song F│       ║ │
│  ║  └──────┘└──────┘  └──────┘└──────┘       ║ │
│  ║                                             ║ │
│  ║  LIGHT ROTATION    NEW MUSIC                ║ │
│  ║  ┌──────┐┌──────┐  ┌──────┐┌──────┐       ║ │
│  ║  │Song I││Song J│  │Song M││Song N│       ║ │
│  ║  └──────┘└──────┘  └──────┘└──────┘       ║ │
│  ║                                             ║ │
│  ║  [Statistics: plays, requests, dedications]║ │
│  ╚═════════════════════════════════════════════╝ │
└──────────────────────────────────────────────────┘
```

## Asset Generation Strategy

### Visual Elements

#### 1. Vinyl Cart Cartridges
**NAB Broadcast Carts** (8-track style):
- Plastic cartridge shells
- Colored labels (red, blue, yellow, green) by genre/rotation
- Typewriter-style text labels
- Artist/Title on front label
- Timing marks on spine
- Worn/aged appearance (coffee stains, wear marks)

**Generation Approach**:
```python
cart_prompt = """
Vintage broadcast cart cartridge for radio station,
{color} plastic shell, white label with typewriter text,
artist: {artist}, title: {title},
professional product photography, slight wear and aging,
1970s radio equipment, studio lighting, realistic
"""

label_design:
  - Station logo (generated once, reused)
  - Artist/Title in Courier typewriter font
  - Timing info (intro/outro cues)
  - Rotation color code
  - Cart number
```

#### 2. Studio Equipment
**Control Room Elements**:
- Mixing console with faders
- VU meters (analog needle style)
- Reel-to-reel tape machine
- Microphone on boom arm
- "ON AIR" illuminated sign
- Cart machine slots (3-slot player)
- Turntable (reference, not primary)
- Clock (analog wall clock showing time)
- Coffee cups, ashtrays (period accurate)

**Generation**:
```yaml
equipment_generation:
  mixing_board:
    prompt: |
      1970s radio station mixing console, professional broadcast equipment,
      multiple faders and knobs, VU meters, patch bay,
      wood grain sides, beige/tan color scheme, realistic lighting,
      worn but functional, studio photography
    
  cart_machine:
    prompt: |
      Triple cart machine, broadcast equipment, three cart slots,
      industrial design, metal construction, LED indicators,
      1970s radio studio equipment, professional product photography
  
  on_air_sign:
    prompt: |
      Illuminated ON AIR sign, red light, industrial design,
      wall-mounted, 1970s radio station, glowing when lit,
      vintage broadcasting equipment
```

#### 3. Request Slips & Office Materials
**Handwritten Request Cards**:
- Index card style
- Handwritten appearance (various handwriting fonts)
- Caller name, phone number
- Song request
- Dedication message
- Time received
- Pink/yellow/white paper colors

**Generation Method**:
- Template-based with programmatic text
- Multiple handwriting-style fonts
- Slight rotation/skew for realism
- Coffee ring stains (random placement)
- Worn edges

```python
def generate_request_slip(request):
    template = random.choice(['pink_slip', 'yellow_slip', 'white_slip'])
    font = random.choice(['handwriting_1', 'handwriting_2', 'script'])
    
    slip = render_template(template)
    add_text(slip, f"From: {request.caller}", position='top', font=font)
    add_text(slip, f"Request: {request.song}", position='middle', font=font)
    add_text(slip, f"For: {request.dedication}", position='bottom', font=font)
    
    # Add aging effects
    add_coffee_stain(slip, opacity=random(0.1, 0.3))
    rotate(slip, angle=random(-5, 5))
    
    return slip
```

#### 4. Background Studio Environment
**Control Room**:
- Wood paneling walls (orange/brown tones)
- Egg crate acoustic foam
- Framed station awards/photos
- Clock on wall
- Warm incandescent lighting
- Carpet floor (70s pattern)

**Window View** (optional):
- See into production studio
- Sound engineer visible
- Additional equipment racks

## Animation Styles

### 1. Cart Loading Animation
**Sequence** (3 seconds):
- Current cart ejects upward (0.5s)
- Slides left out of slot (0.5s)
- Next cart slides in from right (0.5s)
- Drops into play position (0.5s)
- Progress lights illuminate (0.5s)
- VU meters begin responding (0.5s)

```javascript
async function cartTransition(currentSong, nextSong) {
  // Eject current
  await animate(currentCart, { y: -50 }, 500);
  await animate(currentCart, { x: -200, opacity: 0 }, 500);
  
  // Load next
  nextCart.x = 200;
  nextCart.opacity = 0;
  await animate(nextCart, { x: 0, opacity: 1 }, 500);
  await animate(nextCart, { y: 0 }, 500);
  
  // Light up
  progressLight.illuminate();
  vuMeter.startAnimating();
}
```

### 2. Flip Display (Artist/Title)
**Mechanical Flip Clock Style**:
- Individual characters flip like departure boards
- Click sound on each flip
- Sequential flipping (not all at once)
- LED backlighting

```javascript
class FlipDisplay {
  async updateText(newText) {
    for (let i = 0; i < newText.length; i++) {
      if (this.text[i] !== newText[i]) {
        await this.flipCharacter(i, newText[i]);
        await sleep(50); // Stagger effect
      }
    }
  }
  
  async flipCharacter(index, newChar) {
    // Rotate character panel 180 degrees
    await animate(this.panels[index], { 
      rotateX: 180 
    }, 200);
    
    // Change text mid-flip
    this.panels[index].text = newChar;
  }
}
```

### 3. VU Meter Bouncing
**Real-time Audio Response**:
- Analog needle movement with inertia
- Smooth momentum (not instant)
- Peak hold indicators
- Red zone warning (>0dB)

```javascript
class VUMeter {
  update(audioLevel) {
    // Add inertia to needle movement
    const target = audioLevel;
    this.currentLevel += (target - this.currentLevel) * 0.3;
    
    // Animate needle
    this.needle.rotation = mapRange(
      this.currentLevel, 
      -60, 0,    // dB range
      -45, 45    // degrees
    );
    
    // Peak hold
    if (this.currentLevel > this.peak) {
      this.peak = this.currentLevel;
      this.peakHoldTime = 2000; // Hold for 2 seconds
    }
  }
}
```

### 4. On-Air Light Pulsing
```javascript
class OnAirLight {
  animate() {
    if (this.isLive) {
      // Gentle pulsing when on-air
      this.brightness = 0.8 + Math.sin(Date.now() * 0.003) * 0.2;
      this.glow = this.brightness * 20; // Bloom effect
    } else {
      this.brightness = 0;
      this.glow = 0;
    }
  }
}
```

### 5. Request Slip Animations
```javascript
// New request arrives
function addRequest(request) {
  const slip = generateRequestSlip(request);
  
  // Slide in from top
  slip.y = -100;
  slip.opacity = 0;
  
  animate(slip, {
    y: 0,
    opacity: 1,
    rotation: random(-3, 3)  // Slight skew
  }, 500, 'easeOut');
  
  // Stack on top of pile
  requestPile.push(slip);
}

// Request selected
function selectRequest(slip) {
  // Highlight and zoom slightly
  animate(slip, {
    scale: 1.05,
    boxShadow: '0 5px 15px rgba(255,200,0,0.5)'
  }, 200);
}
```

## UI Components

### View States

#### 1. Main Control Room View (Default)
**Layout**:
- Top: ON AIR sign + station ID
- Upper middle: Now playing flip display
- Middle: Cart machine (3-bay)
- Lower middle: VU meters (L/R)
- Bottom: Mixing board controls

**Interactive Elements**:
- Tap cart machine → request line view
- Tap VU meters → audio settings
- Tap mixing board → station manager view
- Long press on-air light → toggle live status

#### 2. Request Line / Music Library
**Two Tabs**:

Tab A - Request Slips:
- Stack of caller requests
- Each shows: caller, song, dedication
- Tap to add to queue
- Search box at bottom

Tab B - Music Library:
- Cart rack view (rows of colored carts)
- Organized by rotation/genre
- Color coded: Red=Heavy, Blue=Medium, Yellow=Light, Green=New
- Virtual scroll through collection
- Search/filter by artist/title

#### 3. Station Manager / Programming
**Cork Board Style**:
- Index cards pinned in sections
- Sections: Heavy/Medium/Light/New rotation
- Drag and drop to reorganize
- Click card for stats (plays, requests)
- Add/remove from rotation

### Component Architecture (React)
```
src/
  components/
    RadioStationUI.jsx       # Main container
    ControlRoom.jsx          # Main DJ booth view
    OnAirLight.jsx           # Illuminated sign
    FlipDisplay.jsx          # Mechanical flip text
    CartMachine.jsx          # 3-bay cart player
    VUMeter.jsx              # Analog meters
    MixingBoard.jsx          # Faders and controls
    RequestLine.jsx          # Caller request interface
    RequestSlip.jsx          # Individual request card
    MusicLibrary.jsx         # Cart rack view
    StationManager.jsx       # Programming interface
    CorkBoard.jsx            # Rotation management
  
  utils/
    audioAnalyzer.js         # For VU meter data
    flipAnimation.js         # Flip display mechanics
    cartManager.js           # Asset loading
```

## Mobile Performance Strategies

### Critical Optimizations

1. **Simplified Cart Animation**
   ```javascript
   const cartAnimation = isMobile ? 'slide' : 'full_3d';
   
   if (isMobile) {
     // Simple 2D slide
     cart.style.transform = `translateX(${x}px)`;
   } else {
     // Full 3D eject/load
     cart.style.transform = `
       translateX(${x}px) 
       translateY(${y}px) 
       rotateX(${angle}deg)
     `;
   }
   ```

2. **Static Equipment Images**
   ```javascript
   // Mobile: Static images of equipment
   // Desktop: Animated/interactive equipment
   
   const mixingBoard = isMobile 
     ? <img src="mixing-board-static.jpg" />
     : <InteractiveMixingBoard />;
   ```

3. **Reduced VU Meter Updates**
   ```javascript
   const vuUpdateRate = isMobile ? 100 : 16; // 10fps vs 60fps
   
   setInterval(() => {
     const level = analyzeAudio();
     updateVUMeters(level);
   }, vuUpdateRate);
   ```

4. **Lazy Load Request Slips**
   ```javascript
   // Only render visible request slips
   const visibleRequests = useVirtualizedList(
     allRequests,
     {
       overscan: 3,
       itemHeight: 120
     }
   );
   ```

5. **Preload Next Cart**
   ```javascript
   // When 30s left, load next cart image
   if (timeRemaining < 30) {
     const nextCart = queue[0];
     preloadImage(`/carts/${nextCart.id}.png`);
   }
   ```

6. **CSS-Only Flip Display (Mobile)**
   ```css
   /* Instead of character-by-character flip */
   .flip-display {
     animation: fadeFlip 0.5s ease-in-out;
   }
   
   @keyframes fadeFlip {
     0% { opacity: 1; transform: scaleY(1); }
     50% { opacity: 0; transform: scaleY(0.5); }
     100% { opacity: 1; transform: scaleY(1); }
   }
   ```

7. **Simplified On-Air Light**
   ```javascript
   if (isMobile) {
     // Simple blink
     onAirLight.style.opacity = isLive ? 1 : 0;
   } else {
     // Smooth pulsing with glow
     onAirLight.style.opacity = 0.8 + pulse * 0.2;
     onAirLight.style.filter = `blur(${glow}px)`;
   }
   ```

8. **Touch Target Sizing**
   ```css
   /* Larger tap targets for mobile */
   .cart-slot {
     min-width: 44px;
     min-height: 44px;
     padding: 12px;
   }
   
   .request-slip {
     min-height: 60px;
     padding: 16px;
   }
   ```

9. **Reduced Visual Complexity**
   ```javascript
   const visualQuality = {
     mobile_low: {
       coffee_stains: false,
       card_rotation: false,
       shadows: 'simple',
       background: 'solid_color'
     },
     
     desktop: {
       coffee_stains: true,
       card_rotation: true,
       shadows: 'realistic',
       background: 'wood_texture'
     }
   };
   ```

10. **Progressive Image Loading**
    ```javascript
    // Load cart images progressively
    async function loadCart(songId) {
      // 1. Show color placeholder
      showPlaceholder(songId, getRotationColor(songId));
      
      // 2. Load thumbnail
      const thumb = await loadImage(`${songId}_thumb.jpg`);
      showBlurred(thumb);
      
      // 3. Load full resolution
      const full = await loadImage(`${songId}_cart.jpg`);
      showFull(full);
    }
    ```

## Performance Budgets

### Target Metrics
- **Initial Load**: < 2 seconds to control room view
- **Cart Swap**: < 1 second animation
- **Flip Display**: < 800ms text change
- **VU Response**: < 50ms latency
- **Memory**: < 100MB on mobile
- **Bundle Size**: < 400KB gzipped

### Visual Complexity Budget
```javascript
const effectBudget = {
  mobile_low: {
    cart_animation: 'slide',
    flip_display: 'fade',
    vu_meters: 'static',
    on_air_light: 'blink',
    request_slips: 'no_rotation',
    background: 'solid'
  },
  
  mobile_high: {
    cart_animation: 'slide_with_scale',
    flip_display: 'flip',
    vu_meters: 'animated_30fps',
    on_air_light: 'pulse',
    request_slips: 'slight_rotation',
    background: 'gradient'
  },
  
  desktop: {
    cart_animation: 'full_3d',
    flip_display: 'mechanical_flip',
    vu_meters: 'realtime_60fps',
    on_air_light: 'glow_pulse',
    request_slips: 'full_skew_shadows',
    background: 'wood_texture'
  }
};
```

## Asset Generation Details

### Broadcast Cart Design

```yaml
cart_generation:
  base_template:
    method: "AI generate once, reuse"
    prompt: |
      Broadcast cart cartridge for radio station, NAB standard,
      plastic shell, professional product photography,
      studio lighting, slight wear, 1970s equipment
  
  color_variants:
    heavy_rotation: "#DC143C"  # Red
    medium_rotation: "#4169E1" # Blue
    light_rotation: "#FFD700"  # Yellow
    new_music: "#32CD32"       # Green
    special: "#FF6347"         # Orange
  
  label_generation:
    method: "Programmatic text on template"
    elements:
      - station_logo: "Top of label"
      - artist: "Courier Bold, 14pt"
      - title: "Courier Regular, 12pt"
      - timing: "Small text, intro/outro cues"
      - cart_number: "Bottom right"
      - rotation_color: "Border color-coded"
    
    aging_effects:
      - slight_yellowing: true
      - edge_wear: true
      - coffee_ring: random(0.3 probability)
      - scratches: random(0.2 probability)
```

### Request Slip Design

```yaml
request_slip_generation:
  templates:
    - pink_while_you_were_out
    - yellow_memo_pad
    - white_index_card
  
  handwriting_fonts:
    - "Permanent Marker"
    - "Indie Flower"
    - "Shadows Into Light"
    - "Caveat"
  
  content_layout:
    from: "Top: 'From: [name] - [phone]'"
    request: "Middle: 'Request: [artist] - [title]'"
    dedication: "Bottom: 'For: [recipient]'"
    time: "Corner: timestamp"
  
  effects:
    rotation: random(-5, 5) degrees
    coffee_stain: 30% probability
    pen_smudge: 20% probability
    crease: 10% probability
```

### Control Room Background

```yaml
studio_environment:
  walls:
    material: "Wood paneling"
    color: "#8B4513" to "#D2691E"  # Brown tones
    texture: "Vertical planks"
  
  acoustic_treatment:
    - "Egg crate foam panels"
    - "Gray/charcoal color"
    - "Random placement"
  
  lighting:
    type: "Warm incandescent"
    temperature: "2700K"
    sources:
      - "Overhead can lights"
      - "Desk lamps"
      - "Equipment indicator LEDs"
  
  props:
    - "Wall clock (analog)"
    - "Framed station awards"
    - "Coffee cups (multiple)"
    - "Ashtray (period accurate)"
    - "Notepad with scribbles"
    - "Station ID cart in holder"
```

## Implementation Phases

### Phase 1: Core Control Room (Week 1-2)
- React component structure
- SSE integration (reuse from other UIs)
- Static control room layout
- Generate equipment images (console, cart machine)
- Basic cart display

### Phase 2: Cart Animation System (Week 2-3)
- Cart loading animation
- Progress indicators
- Asset management for carts
- Generate 50 test cart labels

### Phase 3: Flip Display & VU Meters (Week 3-4)
- Mechanical flip animation
- Web Audio integration for VU meters
- Real-time audio analysis
- On-air light effects

### Phase 4: Request Line Interface (Week 4-5)
- Request slip generation
- Stack animation
- Search functionality
- Touch gestures

### Phase 5: Music Library (Week 5-6)
- Cart rack visualization
- Color coding by rotation
- Virtual scrolling
- Filtering and search

### Phase 6: Asset Generation (Week 6-7)
- Generate cart labels for 10k library
- Batch processing with priorities
- Rotation color assignment
- Quality control

### Phase 7: Mobile Optimization (Week 7-8)
- Simplified animations
- Static equipment fallbacks
- Touch target sizing
- Performance profiling

### Phase 8: Polish & Production (Week 8-9)
- Station manager view
- Statistics and analytics
- Error handling
- Documentation

## Technical Stack

### Frontend
```javascript
{
  "react": "^18",
  "framer-motion": "^10",      // Smooth animations
  "react-spring": "^9",        // Physics animations
  "use-sound": "^4.0",         // Click sounds
  "canvas-confetti": "^1.6"    // Particle effects (optional)
}
```

### AI Generation
- **Stable Diffusion**: Equipment and cart images
- **Photoshop/GIMP**: Label compositing
- **PIL/Canvas**: Programmatic text rendering

### Audio
- **Web Audio API**: VU meter data
- **Tone.js**: Alternative audio analysis

## Key Design Decisions

✅ **Theme**: Behind-the-scenes radio station (1970s-80s)  
✅ **Equipment**: Broadcast carts, mixing console, VU meters  
✅ **Selection**: Request line (caller slips) + cart library  
✅ **Animation**: Cart loading/ejecting, flip display, VU meters  
✅ **Colors**: Warm browns, orange tones, rotation color-coding  
✅ **Typography**: Helvetica, Courier (typewriter), handwritten  
✅ **Mobile**: Simplified animations, larger touch targets  

## Comparison with Other UIs

| Aspect | Jukebox | Dance Club | Festival | Radio Station |
|--------|---------|------------|----------|---------------|
| **Era** | 1970s | 1980s | 1960s | 1970s-80s |
| **Setting** | Diner | Nightclub | Concert | Studio |
| **Format** | 45 singles | 12" vinyl | Album art | Broadcast carts |
| **Selection** | Button grid | Crate dig | Poster wall | Request line |
| **Animation** | Mechanical | Digital | Organic | Electromechanical |
| **Effects** | Realistic | Strobes | Liquid | Professional |
| **Colors** | Chrome/warm | Neon/black | Vibrant | Brown/orange |
| **Vibe** | Nostalgic | Energetic | Trippy | Behind-scenes |

## Integration Checklist

- [ ] Reuse backend SSE infrastructure
- [ ] Add rotation status to song metadata
- [ ] Track last_played timestamps for variety
- [ ] Set up Stable Diffusion for cart generation
- [ ] Design cart label template
- [ ] Create rotation color scheme
- [ ] Generate mixing board image
- [ ] Test flip display animation performance
- [ ] Implement Web Audio API for VU meters
- [ ] Profile mobile cart animation performance
- [ ] Test virtual scrolling with 10k carts

## Radio-Specific Features

### Rotation Management
```javascript
const rotationStatus = {
  heavy: {
    color: '#DC143C',  // Red
    plays_per_day: 12,
    description: 'Current hits'
  },
  medium: {
    color: '#4169E1',  // Blue
    plays_per_day: 6,
    description: 'Recent favorites'
  },
  light: {
    color: '#FFD700',  // Yellow
    plays_per_day: 2,
    description: 'Classic rotation'
  },
  new: {
    color: '#32CD32',  // Green
    plays_per_day: 4,
    description: 'New music testing'
  }
};
```

### Clock & Timing
```javascript
// Hour clock for programming
class StationClock {
  constructor() {
    this.topOfHour = false;    // Legal ID required
    this.bottomOfHour = false; // News/weather break
  }
  
  checkEvents() {
    const minutes = new Date().getMinutes();
    
    if (minutes === 0) {
      this.topOfHour = true;
      // Trigger station ID
    }
    
    if (minutes === 30) {
      this.bottomOfHour = true;
      // Trigger news/weather
    }
  }
}
```

### Request Tracking
```javascript
class RequestSystem {
  trackRequest(songId, caller) {
    // Increment request counter
    song.request_count++;
    
    // Create request slip
    const slip = {
      id: generateId(),
      caller: caller.name,
      phone: caller.phone,
      song: songId,
      dedication: caller.dedication,
      time: new Date()
    };
    
    // Add to queue
    requestQueue.push(slip);
    
    // Notify UI
    emit('new_request', slip);
  }
}
```

## Easter Eggs & Details

**Period-Accurate Details**:
- Coffee cup rings on console
- Handwritten log sheets
- "Dead air" warning if silence detected
- Station jingles between songs
- Weather/traffic updates (text only)
- FCC compliance notices

**Interactive Elements**:
- Click coffee cup → refill animation
- Click clock → show programming schedule
- Long-press on-air light → studio cam view (if available)
- Shake device → cart jam error message

**Sound Effects** (optional):
- Cart click when loading
- Flip display mechanical sound
- VU meter peak warning beep
- Phone ringing for requests
- Air conditioning hum (ambient)
