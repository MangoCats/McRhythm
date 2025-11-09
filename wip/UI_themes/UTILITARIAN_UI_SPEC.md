# Utilitarian Music Player UI - High Level Specification

## Overview
Clean, functional music player interface focused on usability and efficiency. Features standard playback controls, album art display, lyrics viewing, and hierarchical music library browsing. Prioritizes clarity, accessibility, and quick access to essential functions.

## Visual Theme References

### Design Inspiration
- **Modern Music Apps**: Spotify, Apple Music, YouTube Music
- **Material Design**: Clear hierarchy, consistent spacing, readable typography
- **Accessibility First**: High contrast, large touch targets, screen reader support
- **Clean Aesthetics**: Minimal decoration, functional color use
- **Typography**: System fonts for performance and readability

## Integration Points

### Backend Communication
```rust
// Same infrastructure, focused on core playback features
GET /player/state
GET /player/events (SSE)
POST /player/enqueue
POST /player/control

// Extended for rating and control
struct PlaybackControl {
    action: ControlAction,
}

enum ControlAction {
    Play,
    Pause,
    Skip,
    Previous,
    ThumbsUp,      // Mark as favorite / increase play frequency
    ThumbsDown,    // Mark as disliked / decrease play frequency
}

struct SongMetadata {
    song_id: String,
    title: String,
    artist: String,
    album: String,
    album_art_url: Option<String>,
    duration_ms: u64,
    
    // Rating info
    user_rating: Option<Rating>,  // User's thumbs up/down
    play_count: u32,
    last_played: Option<DateTime>,
    
    // Lyrics
    lyrics_available: bool,
    lyrics_url: Option<String>,   // Endpoint to fetch lyrics
}

enum Rating {
    ThumbsUp,
    ThumbsDown,
    Neutral,
}

// Lyrics structure
struct Lyrics {
    song_id: String,
    format: LyricsFormat,
    content: String,              // Plain text or LRC format
}

enum LyricsFormat {
    PlainText,                    // Unsynchronized lyrics
    LRC,                          // Time-synced LRC format
}
```

### Lyrics Display Requirements
```rust
// CRITICAL: Never store or return copyrighted lyrics
// Implementation must:
// 1. Only display lyrics that user has rights to view
// 2. Use licensed lyrics API (Musixmatch, Genius, etc.)
// 3. Or allow user to provide their own lyrics files
// 4. Never scrape or reproduce copyrighted content

// Example integration with licensed API
async fn fetch_lyrics(song_id: &str, api_key: &str) -> Option<Lyrics> {
    // Call licensed API like Musixmatch
    // Returns lyrics only if properly licensed
}
```

## UI Layout

### Main Player View
```
┌─────────────────────────────────────────────────┐
│  ← Back to Library          Queue (5) →         │  <- Header
├─────────────────────────────────────────────────┤
│                                                  │
│              ┌────────────────┐                 │
│              │                │                 │
│              │  Album Art     │  <- Large, centered
│              │  (300x300)     │     Click to expand
│              │                │                 │
│              └────────────────┘                 │
│                                                  │
│         Song Title (Large)                       │
│         Artist Name (Medium)                     │
│         Album Name (Small)                       │
│                                                  │
├─────────────────────────────────────────────────┤
│  [████████████░░░░░░░] 2:34 / 4:15              │  <- Progress bar
├─────────────────────────────────────────────────┤
│                                                  │
│    👍      ⏮️      ⏯️      ⏭️       👎          │  <- Controls
│   (48px)  (48px)  (56px)  (48px)   (48px)      │     Large touch targets
│                                                  │
├─────────────────────────────────────────────────┤
│                                                  │
│  Tabs: [ Info ] [ Lyrics ] [ Queue ]            │
│                                                  │
│  [Tab content area]                             │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Library Browse View
```
┌─────────────────────────────────────────────────┐
│  Library                     [Search 🔍]         │
├─────────────────────────────────────────────────┤
│                                                  │
│  View by: [Artists▼] [Albums] [Songs] [Playlists]│
│                                                  │
├─────────────────────────────────────────────────┤
│                                                  │
│  Artists View (Default):                        │
│                                                  │
│  A                                               │
│  ├─ ABBA                            [>]         │  <- Expand to albums
│  ├─ AC/DC                           [>]         │
│  ├─ Adele                           [>]         │
│                                                  │
│  B                                               │
│  ├─ The Beatles                     [>]         │
│  ├─ Beyoncé                         [>]         │
│                                                  │
│  [Virtual scroll through thousands]             │
│                                                  │
└─────────────────────────────────────────────────┘

When artist expanded:
┌─────────────────────────────────────────────────┐
│  ← Artists                                       │
├─────────────────────────────────────────────────┤
│                                                  │
│  The Beatles (245 songs, 13 albums)             │
│                                                  │
│  Albums:                                         │
│                                                  │
│  ┌────────┐  Abbey Road (1969)                  │
│  │ [art]  │  17 songs                     [+All]│  <- Add all to queue
│  └────────┘  ▸ Come Together                [+] │  <- Add song to queue
│               ▸ Something                   [+] │
│               ▸ Maxwell's Silver Hammer    [+] │
│               [Show all 17...]                  │
│                                                  │
│  ┌────────┐  Sgt. Pepper's... (1967)            │
│  │ [art]  │  13 songs                     [+All]│
│  └────────┘  ▸ Songs listed...                  │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Albums View
```
┌─────────────────────────────────────────────────┐
│  Library > Albums                [Search 🔍]     │
├─────────────────────────────────────────────────┤
│                                                  │
│  Sort by: [Artist▼] [Title] [Year] [Recently Added]│
│                                                  │
│  Grid View:                                     │
│                                                  │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐  │
│  │ Album  │ │ Album  │ │ Album  │ │ Album  │  │
│  │  Art   │ │  Art   │ │  Art   │ │  Art   │  │
│  │        │ │        │ │        │ │        │  │
│  └────────┘ └────────┘ └────────┘ └────────┘  │
│  Title      Title      Title      Title        │
│  Artist     Artist     Artist     Artist        │
│                                                  │
│  [Virtual scroll grid]                          │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Songs View (All Songs)
```
┌─────────────────────────────────────────────────┐
│  Library > All Songs             [Search 🔍]     │
├─────────────────────────────────────────────────┤
│                                                  │
│  Sort by: [Title▼] [Artist] [Album] [Recently Played]│
│  Filter: [All Genres▼] [All Years▼]            │
│                                                  │
│  List View:                                     │
│                                                  │
│  ┌──┬─────────────────────────────────────┬───┐│
│  │▶ │ Song Title                          │ + ││
│  │  │ Artist • Album                      │   ││
│  │  │ 3:45                                │   ││
│  ├──┼─────────────────────────────────────┼───┤│
│  │▶ │ Another Song                        │ + ││
│  │  │ Artist • Album                      │   ││
│  │  │ 4:12                                │   ││
│  └──┴─────────────────────────────────────┴───┘│
│                                                  │
│  ▶ = Play now    + = Add to queue               │
│                                                  │
│  [Virtual scroll list - 10,000 songs]           │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Lyrics Tab
```
┌─────────────────────────────────────────────────┐
│  Now Playing > Lyrics                            │
├─────────────────────────────────────────────────┤
│                                                  │
│  Time-synced lyrics (when available):           │
│                                                  │
│  [Verse 1]                                       │
│  Line of lyrics here                             │
│  Another line of lyrics                          │  <- Highlighted
│  Third line of lyrics                            │     current line
│                                                  │
│  [Chorus]                                        │
│  Chorus lyrics here                              │
│  More chorus lyrics                              │
│                                                  │
│  [Auto-scroll follows playback]                  │
│                                                  │
│  ──────────────────────────                     │
│                                                  │
│  If no lyrics available:                         │
│  ┌─────────────────────────────────────┐        │
│  │  Lyrics not available for this song │        │
│  │                                      │        │
│  │  [ Import Lyrics File ]              │        │
│  │  [ Search Online ]                   │        │
│  └─────────────────────────────────────┘        │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Queue View
```
┌─────────────────────────────────────────────────┐
│  Queue                       [Clear All]         │
├─────────────────────────────────────────────────┤
│                                                  │
│  ⚫ Currently Playing:                           │
│  ┌─────────────────────────────────────┐        │
│  │ Song Title                          │        │
│  │ Artist • Album                      │        │
│  │ 2:34 / 4:15                         │        │
│  └─────────────────────────────────────┘        │
│                                                  │
│  ──────────────────────────                     │
│                                                  │
│  Up Next:                                        │
│                                                  │
│  1. ┌─────────────────────────────────┐         │
│     │ Next Song Title                 │  ✕      │  <- Remove
│     │ Artist • Album                  │  ☰      │  <- Drag handle
│     └─────────────────────────────────┘         │
│                                                  │
│  2. ┌─────────────────────────────────┐         │
│     │ Another Song                    │  ✕  ☰  │
│     │ Artist • Album                  │         │
│     └─────────────────────────────────┘         │
│                                                  │
│  [Reorder by dragging ☰]                        │
│  [Tap ✕ to remove]                              │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Search View
```
┌─────────────────────────────────────────────────┐
│  🔍 [Search library...____________] ✕            │
├─────────────────────────────────────────────────┤
│                                                  │
│  Recent Searches:                                │
│  • "beatles"                                     │
│  • "jazz standards"                              │
│  • "1980s rock"                                  │
│                                                  │
│  ──────────────────────────                     │
│                                                  │
│  After typing "bea":                             │
│                                                  │
│  Artists (3):                                    │
│  • The Beatles (245 songs)              [View]  │
│  • Beach Boys (89 songs)                [View]  │
│  • Beastie Boys (56 songs)              [View]  │
│                                                  │
│  Albums (12):                                    │
│  • Abbey Road - The Beatles             [View]  │
│  • Pet Sounds - Beach Boys              [View]  │
│  [Show all 12 albums...]                         │
│                                                  │
│  Songs (47):                                     │
│  • Come Together - The Beatles            [+]   │
│  • Good Vibrations - Beach Boys           [+]   │
│  • No Sleep Till Brooklyn - Beastie Boys  [+]   │
│  [Show all 47 songs...]                          │
│                                                  │
└─────────────────────────────────────────────────┘
```

## UI Components

### Component Architecture (React)
```
src/
  components/
    PlayerUI.jsx                 # Main container
    
    player/
      NowPlaying.jsx             # Main player view
      AlbumArt.jsx               # Album artwork display
      Controls.jsx               # Play/Pause/Skip/Rating buttons
      ProgressBar.jsx            # Playback progress
      InfoTabs.jsx               # Info/Lyrics/Queue tabs
      LyricsDisplay.jsx          # Lyrics viewer with sync
      QueueView.jsx              # Queue management
    
    library/
      LibraryBrowser.jsx         # Main library view
      ArtistList.jsx             # Artist browsing
      AlbumGrid.jsx              # Album grid view
      SongList.jsx               # Song list view
      SearchBar.jsx              # Search interface
      SearchResults.jsx          # Search results display
    
    common/
      Button.jsx                 # Reusable button
      List.jsx                   # Virtual scrolling list
      Grid.jsx                   # Virtual scrolling grid
      EmptyState.jsx             # No content placeholder
  
  hooks/
    usePlayback.js               # Playback control
    useQueue.js                  # Queue management
    useLibrary.js                # Library data
    useSearch.js                 # Search functionality
    useLyrics.js                 # Lyrics fetching/sync
  
  utils/
    lyricsParser.js              # Parse LRC format
    lyricsSync.js                # Sync to playback
    libraryHelpers.js            # Sorting, filtering
```

## Key Features

### 1. Playback Controls
```javascript
const Controls = ({ playbackState, onControl }) => {
  return (
    <div className="controls">
      <button 
        onClick={() => onControl('thumbs_up')}
        className={playbackState.userRating === 'ThumbsUp' ? 'active' : ''}
        aria-label="Like this song"
      >
        👍
      </button>
      
      <button 
        onClick={() => onControl('previous')}
        aria-label="Previous song"
      >
        ⏮️
      </button>
      
      <button 
        onClick={() => onControl(playbackState.isPlaying ? 'pause' : 'play')}
        className="primary-button"
        aria-label={playbackState.isPlaying ? 'Pause' : 'Play'}
      >
        {playbackState.isPlaying ? '⏸️' : '▶️'}
      </button>
      
      <button 
        onClick={() => onControl('skip')}
        aria-label="Skip to next song"
      >
        ⏭️
      </button>
      
      <button 
        onClick={() => onControl('thumbs_down')}
        className={playbackState.userRating === 'ThumbsDown' ? 'active' : ''}
        aria-label="Dislike this song"
      >
        👎
      </button>
    </div>
  );
};
```

### 2. Lyrics Display with Sync
```javascript
class LyricsSync {
  constructor(lyrics, format) {
    this.lyrics = lyrics;
    this.format = format;
    this.lines = this.parseLyrics();
  }
  
  parseLyrics() {
    if (this.format === 'LRC') {
      // Parse LRC format: [mm:ss.xx] Lyric text
      return this.lyrics.split('\n').map(line => {
        const match = line.match(/\[(\d{2}):(\d{2})\.(\d{2})\](.*)/);
        if (match) {
          const [, min, sec, ms, text] = match;
          const timeMs = (parseInt(min) * 60 + parseInt(sec)) * 1000 + parseInt(ms) * 10;
          return { time: timeMs, text: text.trim() };
        }
        return null;
      }).filter(Boolean);
    } else {
      // Plain text: no sync
      return this.lyrics.split('\n').map(text => ({ time: null, text }));
    }
  }
  
  getCurrentLine(currentTimeMs) {
    if (this.format === 'PlainText') return 0;
    
    // Find the line that should be highlighted
    for (let i = this.lines.length - 1; i >= 0; i--) {
      if (this.lines[i].time <= currentTimeMs) {
        return i;
      }
    }
    return 0;
  }
}

const LyricsDisplay = ({ lyrics, currentTime }) => {
  const sync = useMemo(() => new LyricsSync(lyrics.content, lyrics.format), [lyrics]);
  const currentLine = sync.getCurrentLine(currentTime);
  
  const lyricsRef = useRef();
  
  // Auto-scroll to current line
  useEffect(() => {
    if (lyricsRef.current) {
      const lineElement = lyricsRef.current.children[currentLine];
      if (lineElement) {
        lineElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }
  }, [currentLine]);
  
  return (
    <div className="lyrics-display" ref={lyricsRef}>
      {sync.lines.map((line, i) => (
        <p 
          key={i}
          className={i === currentLine ? 'current' : ''}
        >
          {line.text}
        </p>
      ))}
    </div>
  );
};
```

### 3. Efficient Library Browsing
```javascript
// Virtual scrolling for large lists
import { FixedSizeList } from 'react-window';

const SongList = ({ songs, onPlay, onEnqueue }) => {
  const Row = ({ index, style }) => {
    const song = songs[index];
    
    return (
      <div style={style} className="song-row">
        <button 
          onClick={() => onPlay(song.id)}
          aria-label={`Play ${song.title}`}
        >
          ▶️
        </button>
        
        <div className="song-info">
          <div className="song-title">{song.title}</div>
          <div className="song-meta">
            {song.artist} • {song.album}
          </div>
        </div>
        
        <div className="song-duration">
          {formatDuration(song.duration_ms)}
        </div>
        
        <button 
          onClick={() => onEnqueue(song.id)}
          aria-label={`Add ${song.title} to queue`}
        >
          +
        </button>
      </div>
    );
  };
  
  return (
    <FixedSizeList
      height={600}
      itemCount={songs.length}
      itemSize={80}
      width="100%"
    >
      {Row}
    </FixedSizeList>
  );
};
```

### 4. Search Implementation
```javascript
const useSearch = (library) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState(null);
  
  // Debounce search
  useEffect(() => {
    if (!query) {
      setResults(null);
      return;
    }
    
    const timer = setTimeout(() => {
      const searchResults = performSearch(library, query);
      setResults(searchResults);
    }, 300);
    
    return () => clearTimeout(timer);
  }, [query, library]);
  
  return { query, setQuery, results };
};

function performSearch(library, query) {
  const lowerQuery = query.toLowerCase();
  
  // Search artists
  const artists = library.artists.filter(artist =>
    artist.name.toLowerCase().includes(lowerQuery)
  ).slice(0, 10);
  
  // Search albums
  const albums = library.albums.filter(album =>
    album.title.toLowerCase().includes(lowerQuery) ||
    album.artist.toLowerCase().includes(lowerQuery)
  ).slice(0, 10);
  
  // Search songs
  const songs = library.songs.filter(song =>
    song.title.toLowerCase().includes(lowerQuery) ||
    song.artist.toLowerCase().includes(lowerQuery) ||
    song.album.toLowerCase().includes(lowerQuery)
  ).slice(0, 20);
  
  return { artists, albums, songs };
}
```

### 5. Queue Management
```javascript
const QueueView = ({ queue, currentSong, onRemove, onReorder }) => {
  const [items, setItems] = useState(queue);
  
  const handleDragEnd = (result) => {
    if (!result.destination) return;
    
    const reordered = Array.from(items);
    const [removed] = reordered.splice(result.source.index, 1);
    reordered.splice(result.destination.index, 0, removed);
    
    setItems(reordered);
    onReorder(reordered);
  };
  
  return (
    <div className="queue-view">
      <div className="current-song">
        <h3>⚫ Currently Playing</h3>
        <SongCard song={currentSong} />
      </div>
      
      <div className="up-next">
        <h3>Up Next ({items.length})</h3>
        
        <DragDropContext onDragEnd={handleDragEnd}>
          <Droppable droppableId="queue">
            {(provided) => (
              <div 
                {...provided.droppableProps}
                ref={provided.innerRef}
              >
                {items.map((song, index) => (
                  <Draggable 
                    key={song.id} 
                    draggableId={song.id} 
                    index={index}
                  >
                    {(provided) => (
                      <div
                        ref={provided.innerRef}
                        {...provided.draggableProps}
                        className="queue-item"
                      >
                        <div {...provided.dragHandleProps}>
                          ☰
                        </div>
                        
                        <SongCard song={song} />
                        
                        <button 
                          onClick={() => onRemove(song.id)}
                          aria-label="Remove from queue"
                        >
                          ✕
                        </button>
                      </div>
                    )}
                  </Draggable>
                ))}
                {provided.placeholder}
              </div>
            )}
          </Droppable>
        </DragDropContext>
      </div>
    </div>
  );
};
```

## Mobile Performance Strategies

### Critical Optimizations

1. **Virtual Scrolling**
   ```javascript
   // Use react-window for all lists
   // Only render visible items
   // Handles 10,000+ items efficiently
   
   import { FixedSizeList, VariableSizeGrid } from 'react-window';
   ```

2. **Image Loading**
   ```javascript
   // Lazy load album art
   const AlbumArt = ({ url, alt }) => {
     return (
       <img 
         src={url}
         alt={alt}
         loading="lazy"
         decoding="async"
         onError={(e) => {
           e.target.src = '/placeholder-album.png';
         }}
       />
     );
   };
   ```

3. **Search Debouncing**
   ```javascript
   // Wait 300ms after user stops typing
   const debouncedSearch = useDebounce(searchQuery, 300);
   ```

4. **Memoization**
   ```javascript
   // Cache expensive computations
   const sortedSongs = useMemo(() => 
     songs.sort((a, b) => a.title.localeCompare(b.title)),
     [songs]
   );
   ```

5. **Code Splitting**
   ```javascript
   // Lazy load heavy components
   const LyricsDisplay = lazy(() => import('./LyricsDisplay'));
   const AlbumGrid = lazy(() => import('./AlbumGrid'));
   ```

6. **Touch Optimization**
   ```css
   /* Larger touch targets on mobile */
   @media (max-width: 768px) {
     .button {
       min-width: 44px;
       min-height: 44px;
       padding: 12px;
     }
   }
   ```

7. **Reduced Motion**
   ```javascript
   const prefersReducedMotion = 
     window.matchMedia('(prefers-reduced-motion: reduce)').matches;
   
   const transition = prefersReducedMotion 
     ? { duration: 0 } 
     : { duration: 300 };
   ```

8. **Progressive Enhancement**
   ```javascript
   // Check for feature support
   const supportsIntersectionObserver = 'IntersectionObserver' in window;
   
   if (supportsIntersectionObserver) {
     useLazyLoading();
   } else {
     loadAllImages();
   }
   ```

9. **Service Worker Caching**
   ```javascript
   // Cache album art and UI assets
   self.addEventListener('fetch', (event) => {
     if (event.request.url.includes('/album-art/')) {
       event.respondWith(
         caches.match(event.request)
           .then(response => response || fetch(event.request))
       );
     }
   });
   ```

10. **Efficient State Updates**
    ```javascript
    // Batch updates, avoid unnecessary re-renders
    const [state, dispatch] = useReducer(reducer, initialState);
    
    // Use React.memo for components
    const SongRow = React.memo(({ song }) => {
      // Only re-renders if song changes
    });
    ```

## Performance Budgets

### Target Metrics
- **Initial Load**: < 2 seconds to playback controls
- **Library Browse**: < 100ms to show results
- **Search**: < 300ms response time
- **Song Change**: < 200ms to update UI
- **Lyrics Sync**: < 50ms update latency
- **Memory**: < 80MB on mobile
- **Bundle Size**: < 300KB gzipped

## Accessibility Features

### WCAG 2.1 Level AA Compliance

```javascript
// Keyboard navigation
const Controls = () => {
  const handleKeyPress = (e, action) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onControl(action);
    }
  };
  
  return (
    <button
      onClick={() => onControl('play')}
      onKeyPress={(e) => handleKeyPress(e, 'play')}
      aria-label="Play"
      tabIndex={0}
    >
      ▶️
    </button>
  );
};

// Screen reader support
<div role="region" aria-label="Now Playing">
  <h2 id="song-title">{song.title}</h2>
  <p id="song-artist">{song.artist}</p>
</div>

// High contrast mode support
@media (prefers-contrast: high) {
  .button {
    border: 2px solid currentColor;
  }
}

// Focus indicators
.button:focus-visible {
  outline: 3px solid #4A90E2;
  outline-offset: 2px;
}
```

## Responsive Design

### Breakpoints
```css
/* Mobile first approach */
.player-layout {
  /* Mobile: Single column */
  display: flex;
  flex-direction: column;
}

/* Tablet: 768px+ */
@media (min-width: 768px) {
  .player-layout {
    /* Two column layout */
    display: grid;
    grid-template-columns: 300px 1fr;
  }
}

/* Desktop: 1024px+ */
@media (min-width: 1024px) {
  .player-layout {
    /* Three column with sidebar */
    grid-template-columns: 250px 1fr 300px;
  }
}
```

## Implementation Phases

### Phase 1: Core Player (Week 1)
- Basic playback controls (Play/Pause/Skip)
- Album art display
- Progress bar
- SSE integration for playback updates
- Thumbs up/down rating system

### Phase 2: Library Browsing (Week 1-2)
- Artist list with virtual scrolling
- Album grid view
- Song list view
- Basic navigation between views

### Phase 3: Search (Week 2)
- Search bar component
- Real-time search with debouncing
- Results categorization (Artists/Albums/Songs)
- Recent searches

### Phase 4: Queue Management (Week 3)
- Queue view
- Add/remove songs
- Drag-and-drop reordering
- Clear queue

### Phase 5: Lyrics Integration (Week 3-4)
- Lyrics display component
- LRC format parser
- Time-synced highlighting
- Auto-scroll
- Licensed API integration

### Phase 6: Mobile Optimization (Week 4)
- Responsive layouts
- Touch gestures
- Performance profiling
- Accessibility testing

### Phase 7: Polish (Week 5)
- Error handling
- Loading states
- Empty states
- Animations and transitions
- Cross-browser testing

## Technical Stack

### Frontend
```javascript
{
  "react": "^18",
  "react-window": "^1.8",          // Virtual scrolling
  "react-beautiful-dnd": "^13",    // Drag and drop
  "date-fns": "^2.30",             // Time formatting
  "classnames": "^2.3",            // Conditional CSS classes
}
```

### Styling
- **CSS Modules** or **Styled Components**
- **System fonts** for performance
- **CSS Grid** and **Flexbox** for layouts
- **CSS Custom Properties** for theming

## Key Design Decisions

✅ **Approach**: Utilitarian, function-over-form  
✅ **Navigation**: Hierarchical (Artist → Album → Song)  
✅ **Controls**: Standard playback + rating (thumbs up/down)  
✅ **Display**: Album art + metadata + lyrics  
✅ **Search**: Real-time, categorized results  
✅ **Queue**: Visual list with drag-to-reorder  
✅ **Accessibility**: WCAG 2.1 Level AA compliant  
✅ **Performance**: Virtual scrolling, lazy loading  

## Integration Checklist

- [ ] Reuse backend SSE infrastructure
- [ ] Add thumbs up/down rating endpoints
- [ ] Implement lyrics API integration (licensed)
- [ ] Add user rating storage
- [ ] Test with 10k song library
- [ ] Verify virtual scrolling performance
- [ ] Accessibility audit with screen reader
- [ ] Test keyboard navigation
- [ ] Verify touch target sizes (mobile)
- [ ] Performance profiling on various devices

## Lyrics API Integration

### Licensed Lyrics Providers

**Musixmatch API**:
```javascript
const fetchLyrics = async (songId, isrc) => {
  const response = await fetch(
    `https://api.musixmatch.com/ws/1.1/matcher.lyrics.get`,
    {
      params: {
        q_track: song.title,
        q_artist: song.artist,
        apikey: MUSIXMATCH_API_KEY
      }
    }
  );
  
  const data = await response.json();
  return data.message.body.lyrics;
};
```

**Genius API** (metadata only, no full lyrics):
```javascript
// Genius provides annotations and metadata
// Full lyrics require user to view on Genius.com
const fetchGeniusMetadata = async (songTitle, artist) => {
  // Returns link to lyrics page, not lyrics themselves
};
```

**User-Provided Lyrics**:
```javascript
// Allow users to import their own .lrc files
const importLyricsFile = (file) => {
  const reader = new FileReader();
  reader.onload = (e) => {
    const content = e.target.result;
    saveLyricsLocally(songId, content);
  };
  reader.readAsText(file);
};
```

## Copyright Compliance

### Critical Requirements

**Never display unlicensed lyrics**:
- Only show lyrics from licensed APIs
- Or allow users to provide their own lyrics files
- Never scrape lyrics from websites
- Never reproduce copyrighted lyrics in code or documentation

**Display Attribution**:
```javascript
<div className="lyrics-footer">
  <p>Lyrics provided by Musixmatch</p>
  <a href={musixmatchUrl}>View on Musixmatch</a>
</div>
```

**Handle Missing Lyrics Gracefully**:
```javascript
{!lyrics && (
  <div className="no-lyrics">
    <p>Lyrics not available for this song</p>
    <button onClick={importLyricsFile}>
      Import Lyrics File (.lrc)
    </button>
  </div>
)}
```

## Comparison with Themed UIs

| Aspect | Jukebox | Dance Club | Festival | Radio | Celtic | Utilitarian |
|--------|---------|------------|----------|-------|--------|-------------|
| **Focus** | Nostalgia | Energy | Art | Broadcast | Nature | Function |
| **Complexity** | Medium | High | High | Medium | High | Low |
| **Animation** | Moderate | Heavy | Heavy | Moderate | Moderate | Minimal |
| **Learning Curve** | Low | Medium | Medium | Low | Low | Minimal |
| **Performance** | Good | Medium | Medium | Good | Good | Excellent |
| **Use Case** | Fun | Party | Relaxation | Retro | Ambient | Daily driver |

The utilitarian UI serves as the practical, efficient option for users who prioritize functionality over aesthetics, while the themed UIs provide immersive experiences for specific moods or contexts.
