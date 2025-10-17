# WKMP Audio Player Documentation

## Audio Playback Architecture Documentation

This directory contains comprehensive documentation for the WKMP Audio Player's playback architecture designs.

### 📚 Document Index

1. **[architecture-comparison.md](archive/ARCH003-architecture_comparison.md)** - **START HERE**
   - Side-by-side comparison of Dual Pipeline vs Single Stream approaches
   - Detailed analysis of implementation complexity, performance, and deployment
   - Recommendation table with scores
   - Best for: Decision making and understanding trade-offs

2. **[single-stream-design.md](SPEC013-single_stream_playback.md)** - **CORE DESIGN DESCRIPTION**
   - Manual buffer management with sample-accurate crossfading
   - Pure Rust implementation using symphonia, rubato, and cpal
   - Detailed component design and implementation phases
   - Status: 📋 Proposed (not yet implemented)

### 📜 Reading Guide

#### For Developers (Implementing Single Stream)
1. Read: `single-stream-design.md`
2. Review: Component Structure and Data Flow
3. Follow: Implementation Phases (Week 1-4 plan)
4. Reference: Code examples and algorithm pseudocode

#### For Audio Engineers
1. Read: Crossfade Quality sections in `architecture-comparison.md`
2. Review: Fade curve algorithms in `single-stream-design.md`
3. Compare: Timing precision (sample-accurate vs property-based)

### ðŸ—ï¸ Architecture Diagram

#### Single Stream (Manual Buffers)
```
Application
├── Decoder Pool (parallel decoding)
│   └── Uses symphonia + rubato
├── Passage Buffer Manager
│   └── 15-second PCM buffers per passage
├── Crossfade Mixer
│   └── Sample-accurate mixing with fade curves
└── Audio Output (cpal)
    └── Ring buffer → audio device
```


### ðŸš€ Getting Started

**To plan Single Stream migration:**
```bash
# Read the detailed design
cat docs/single-stream-design.md

# Review comparison
cat docs/architecture-comparison.md

# Check implementation phases
grep -A 20 "## Implementation Phases" docs/single-stream-design.md
```

### ðŸ¤ Contributing

When updating these documents:
1. Keep version numbers in sync
2. Update "Last Updated" dates
3. Cross-reference related documents
4. Maintain code examples with actual implementation
5. Update this README if adding new documents

### 📝§ Questions?

For technical questions about:
- **Single Stream**: See `single-stream-design.md` "Challenges and Solutions"

---

**Documentation Set Version:** 1.0
**Created:** 2025-10-16
**Maintained By:** WKMP Development Team
