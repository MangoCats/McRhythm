# McRhythm
Auto DJ for playing your music library

## Documentation

- **[Requirements](requirements.md)** - Complete feature specifications and requirements
- **[Architecture](architecture.md)** - High-level system architecture and component design
- **[Database Schema](database_schema.md)** - Complete SQLite database schema
- **[Implementation Order](implementation_order.md)** - Phased development plan
- **[Event System](event_system.md)** - Event-driven architecture and communication patterns
- **[Crossfade Design](crossfade.md)** - Crossfade timing and behavior specifications
- **[Musical Flavor](musical_flavor.md)** - Musical flavor characterization system
- **[Coding Conventions](coding_conventions.md)** - Code organization and style guidelines
- **[Requirements Enumeration](requirements_enumeration.md)** - Requirement ID scheme specification

## Overview

McRhythm is a music player that automatically selects passages to play based on user-configured musical flavor preferences by time of day, using cooldown-based probability calculations and AcousticBrainz musical characterization data.

Built with Rust, GStreamer, SQLite, and Tauri.
