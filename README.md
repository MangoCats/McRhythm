# WKMP
Wonderfully Kinetic Music Player

## Overview

WKMP is a music player that plays locally stored audio files to local audio output devices.  Passages from the audio files may be automatically selected to play based on user-configured musical flavor preferences using AcousticBrainz musical characterization data and other algorithms.

The system can work as a simple music file player, or build up to a hourly, daily, weekly, annually programed music source inspired by FM radio stations of the 1970s.

Built with Rust and SQLite using a **5-microservice architecture** for modularity and version flexibility.

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

## Future Features & Extensibility

WKMP's microservices architecture currently consists of **5 core modules**, but is designed to support additional modules for future functionality:

### Potential Future Modules

**News and Weather Integration**
- Text-to-speech news and weather segments between songs on a schedule
- Third-party news aggregation APIs:
  - **NewsData.io**: Integrates content from thousands of trusted news sources and can be filtered by country (e.g., US), language, keyword, and more. You can access real-time and historical data through a simple JSON format.
  - **News API**: A popular choice that indexes articles from over 80,000 worldwide sources. It supports location-based filtering, provides articles in JSON format, and is well-documented.
  - **World News API**: Offers access to news from thousands of sources and can be filtered by country or region, as well as by language and category. It provides a free tier for low-volume personal projects.

**Additional Module Ideas**
- **Alternative UI Implementations**: Mobile-optimized interfaces, accessibility-focused designs, minimal kiosk mode
- **External Control Protocols**: MPD (Music Player Daemon) compatibility, voice assistant integration (Alexa, Google Assistant), MQTT control for home automation
- **Podcast and Audiobook Support**: Extending beyond music to spoken-word content
- **Streaming Service Integration**: Supplementing local library with streaming sources
- **ListenBrainz Integration**: Social sharing of listening data

The architecture's HTTP/SSE-based communication and shared database design enable new modules to be added without modifying existing services. See [Architecture - Extensibility Principle](docs/architecture.md#extensibility-principle) for technical details. 
