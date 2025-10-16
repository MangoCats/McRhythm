# WKMP
Auto DJ for playing your music library

## Overview

WKMP is a music player that plays locally stored audio files to local audio output devices.  Passages from the audio files are automatically selected to play based on user-configured musical flavor preferences using AcousticBrainz musical characterization data and other algorithms.

The system can work as a simple music file player, or build up to a hourly, daily, weekly, annually programed music source inspired by FM radio stations of the 1970s.

Built with Rust, GStreamer and SQLite.

## Documentation

**📋 Start Here:** [Document Hierarchy](docs/document_hierarchy.md) - Documentation framework and governance (Tier 0)

### Core Documentation

- **[Requirements](docs/requirements.md)** - Complete feature specifications and requirements (Tier 1 - Authoritative)
- **[Entity Definitions](docs/entity_definitions.md)** - Core entity terminology (Track, Recording, Song, Passage) (Tier 1 - Authoritative)
- **[Architecture](docs/architecture.md)** - High-level system architecture and component design (Tier 2 - Design)
- **[Event System](docs/event_system.md)** - Event-driven architecture and communication patterns (Tier 2 - Design)
- **[Crossfade Design](docs/crossfade.md)** - Crossfade timing and behavior specifications (Tier 2 - Design)
- **[Musical Flavor](docs/musical_flavor.md)** - Musical flavor characterization system (Tier 2 - Design)
- **[Database Schema](docs/database_schema.md)** - Complete SQLite database schema (Tier 3 - Implementation)
- **[Coding Conventions](docs/coding_conventions.md)** - Code organization and style guidelines (Tier 3 - Implementation)
- **[Implementation Order](docs/implementation_order.md)** - Phased development plan (Tier 4 - Execution)

### Process & Standards

- **[Document Hierarchy](docs/document_hierarchy.md)** - Documentation framework, relationships, and change control (Tier 0 - Governance)
- **[Requirements Enumeration](docs/requirements_enumeration.md)** - Requirement ID scheme specification (Cross-cutting)

## Idea for a future feature

Text to speech news and weather read between songs on a schedule

Third-party news aggregation APIs

These are the most suitable option for comprehensive local coverage because they pull from a vast network of sources, including local affiliates and online publications.
- NewsData.io: Integrates content from thousands of trusted news sources and can be filtered by country (e.g., US), language, keyword, and more. You can access real-time and historical data through a simple JSON format.
- News API: A popular choice that indexes articles from over 80,000 worldwide sources. It supports location-based filtering, provides articles in JSON format, and is well-documented.
- World News API: Offers access to news from thousands of sources and can be filtered by country or region, as well as by language and category. It provides a free tier for low-volume personal projects. 
