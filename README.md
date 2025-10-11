# WKMP
Auto DJ for playing your music library

## Documentation

**📋 Start Here:** [Document Hierarchy](document_hierarchy.md) - Documentation framework and governance (Tier 0)

### Core Documentation

- **[Requirements](requirements.md)** - Complete feature specifications and requirements (Tier 1 - Authoritative)
- **[Entity Definitions](entity_definitions.md)** - Core entity terminology (Track, Recording, Song, Passage) (Tier 1 - Authoritative)
- **[Architecture](architecture.md)** - High-level system architecture and component design (Tier 2 - Design)
- **[Event System](event_system.md)** - Event-driven architecture and communication patterns (Tier 2 - Design)
- **[Crossfade Design](crossfade.md)** - Crossfade timing and behavior specifications (Tier 2 - Design)
- **[Musical Flavor](musical_flavor.md)** - Musical flavor characterization system (Tier 2 - Design)
- **[Database Schema](database_schema.md)** - Complete SQLite database schema (Tier 3 - Implementation)
- **[Coding Conventions](coding_conventions.md)** - Code organization and style guidelines (Tier 3 - Implementation)
- **[Implementation Order](implementation_order.md)** - Phased development plan (Tier 4 - Execution)

### Process & Standards

- **[Document Hierarchy](document_hierarchy.md)** - Documentation framework, relationships, and change control (Tier 0 - Governance)
- **[Requirements Enumeration](requirements_enumeration.md)** - Requirement ID scheme specification (Cross-cutting)

## Overview

WKMP is a music player that automatically selects passages to play based on user-configured musical flavor preferences by time of day, using cooldown-based probability calculations and AcousticBrainz musical characterization data.

Built with Rust, GStreamer, SQLite, and Tauri.
