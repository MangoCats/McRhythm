# McRhythm
Auto DJ for playing your music library

## Documentation

- **[Requirements](requirements.md)** - Complete feature specifications and requirements
- **[Architecture](architecture.md)** - High-level system architecture and component design
- **[Database Schema](database_schema.md)** - Complete SQLite database schema
- **[Implementation Order](implementation_order.md)** - Phased development plan

## Overview

McRhythm is a music player that automatically selects passages to play based on user-configured musical flavor preferences by time of day, using cooldown-based probability calculations and AcousticBrainz musical characterization data.

Built with Rust, GStreamer, SQLite, and Tauri.
