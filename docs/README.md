# WKMP Audio Player Documentation

## Audio Playback Architecture Documentation

This directory contains comprehensive documentation for the WKMP Audio Player's playback architecture designs.

### 📚 Document Index

1. **[architecture-comparison.md](architecture-comparison.md)** - **START HERE**
   - Side-by-side comparison of Dual Pipeline vs Single Stream approaches
   - Detailed analysis of implementation complexity, performance, and deployment
   - Recommendation table with scores
   - Best for: Decision making and understanding trade-offs

2. **[dual-pipeline-design.md](dual-pipeline-design.md)** - **CURRENT IMPLEMENTATION**
   - GStreamer-based dual pipeline architecture (currently implemented)
   - Technical details of bins, audiomixer, and state management
   - Lessons learned from implementation
   - Status: ✅ Working (basic playback functional)

3. **[single-stream-design.md](single-stream-design.md)** - **PROPOSED ALTERNATIVE**
   - Manual buffer management with sample-accurate crossfading
   - Pure Rust implementation using symphonia, rubato, and cpal
   - Detailed component design and implementation phases
   - Status: 📋 Proposed (not yet implemented)

4. **[../single_stream_crossfade.txt](../single_stream_crossfade.txt)** - **ORIGINAL CONCEPT**
   - Original high-level description of single stream approach
   - Foundation for the detailed single-stream-design.md

### 🎯 Quick Reference

#### Which approach should I use?

| If you need... | Use this approach |
|---------------|-------------------|
| Working playback **right now** | **Dual Pipeline** (current) |
| **Sample-accurate crossfading** (~0.02ms precision) | **Single Stream** (proposed) |
| **Low memory usage** (~31 MB vs 170 MB) | **Single Stream** (proposed) |
| **No external dependencies** (static binary) | **Single Stream** (proposed) |
| **Battle-tested framework** (GStreamer) | **Dual Pipeline** (current) |
| **Quick prototyping** (500 LOC vs 2000-3000 LOC) | **Dual Pipeline** (current) |

#### Overall Recommendation

**For WKMP Audio Player:** The **Single Stream** approach is objectively better for professional audio crossfading requirements:
- Sample-accurate mixing (0.02ms vs 10-50ms)
- 70-80% memory reduction (31 MB vs 170 MB)
- Pure Rust (single binary vs framework dependency)
- Full control over audio pipeline

**Pragmatic Path:**
1. ✅ Keep Dual Pipeline working for immediate functionality
2. 🚧 Implement Single Stream approach (2-4 weeks estimated)
3. 🧪 Validate crossfade quality and performance
4. 🚀 Migrate to Single Stream for production

### 📖 Reading Guide

#### For Decision Makers
1. Read: `architecture-comparison.md` (Quick Reference Table)
2. Review: Recommendation section
3. Consider: Timeline and resource trade-offs

#### For Developers (Maintaining Current Implementation)
1. Read: `dual-pipeline-design.md`
2. Focus on: "Critical Lessons Learned" section
3. Reference: Implementation details and state management

#### For Developers (Implementing Single Stream)
1. Read: `single-stream-design.md`
2. Review: Component Structure and Data Flow
3. Follow: Implementation Phases (Week 1-4 plan)
4. Reference: Code examples and algorithm pseudocode

#### For Audio Engineers
1. Read: Crossfade Quality sections in `architecture-comparison.md`
2. Review: Fade curve algorithms in `single-stream-design.md`
3. Compare: Timing precision (sample-accurate vs property-based)

### 📊 Key Metrics Comparison

| Metric | Dual Pipeline | Single Stream |
|--------|---------------|---------------|
| **Crossfade Precision** | ~10-50ms | ~0.02ms |
| **Memory Usage** | ~170 MB | ~31 MB |
| **Implementation Time** | 2-3 days | 2-4 weeks |
| **Binary Size** | ~110-210 MB | ~15 MB |
| **Dependencies** | GStreamer framework | Pure Rust libs |
| **Cross-Platform** | Good (requires runtime) | Excellent (static) |
| **Debugging** | Hard (C framework) | Easy (Rust code) |

### 🏗️ Architecture Diagrams

#### Dual Pipeline (GStreamer)
```
Main Pipeline
├── Pipeline A (Bin)
│   ├── filesrc → decodebin → audioconvert → audioresample → volume
│   └── (controlled via volume property)
├── Pipeline B (Bin)
│   ├── filesrc → decodebin → audioconvert → audioresample → volume
│   └── (controlled via volume property)
├── audiomixer (mixes A + B)
├── master_volume (global control)
└── autoaudiosink (output to device)
```

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

### 🔧 File Locations

**Current Implementation:**
- `wkmp-ap/src/playback/pipeline/dual.rs` - Dual pipeline implementation
- `wkmp-ap/src/playback/engine.rs` - Playback engine integration
- `wkmp-ap/src/playback/monitor.rs` - Background monitoring tasks

**Future Implementation (Single Stream):**
```
wkmp-ap/src/playback/pipeline/single_stream/
├── mod.rs             # SingleStreamPipeline
├── decoder.rs         # DecoderPool
├── buffer.rs          # PassageBufferManager
├── mixer.rs           # CrossfadeMixer
├── output.rs          # AudioOutput
└── curves.rs          # Fade curve algorithms
```

### 🚀 Getting Started

**To understand the current system:**
```bash
# Read the current implementation design
cat docs/dual-pipeline-design.md

# View the actual code
cat wkmp-ap/src/playback/pipeline/dual.rs
```

**To plan Single Stream migration:**
```bash
# Read the detailed design
cat docs/single-stream-design.md

# Review comparison
cat docs/architecture-comparison.md

# Check implementation phases
grep -A 20 "## Implementation Phases" docs/single-stream-design.md
```

### 📝 Document Status

| Document | Version | Status | Last Updated |
|----------|---------|--------|--------------|
| dual-pipeline-design.md | 1.1 | ✅ Current Implementation | 2025-10-16 |
| single-stream-design.md | 1.0 | 📋 Proposed Design | 2025-10-16 |
| architecture-comparison.md | 1.0 | 📊 Analysis Complete | 2025-10-16 |

### 🤝 Contributing

When updating these documents:
1. Keep version numbers in sync
2. Update "Last Updated" dates
3. Cross-reference related documents
4. Maintain code examples with actual implementation
5. Update this README if adding new documents

### 📧 Questions?

For technical questions about:
- **Dual Pipeline**: See `dual-pipeline-design.md` "Critical Lessons Learned"
- **Single Stream**: See `single-stream-design.md` "Challenges and Solutions"
- **Trade-offs**: See `architecture-comparison.md` "Use Case Analysis"

---

**Documentation Set Version:** 1.0
**Created:** 2025-10-16
**Maintained By:** WKMP Development Team
