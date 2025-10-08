# McRhythm Multi-User Coordination

**🤝 TIER 2 - DESIGN SPECIFICATION**

Defines the mechanism for coordinating actions from multiple users to ensure a consistent and predictable experience. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [API Design](api_design.md) | [Event System](event_system.md)

---

## Overview

**[MUC-OV-010]** McRhythm allows multiple users to interact with the same playback queue and state via the WebUI. This document specifies the design for handling concurrent user actions to prevent race conditions and ensure predictable behavior.

**[MUC-OV-020]** The core of the multi-user coordination is the `UserAction` event, which is broadcast to all components and connected clients. This allows for both client-side UI updates and server-side logic to handle edge cases.

## The `UserAction` Event

**[MUC-EVT-010]** As defined in the [Event System](event_system.md), the `UserAction` event is central to multi-user coordination.

```rust
/// Emitted when user performs an action
UserAction {
    action: UserActionType,
    user_id: String, // User's persistent UUID
    timestamp: SystemTime,
}

/// Why a user action occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionType {
    Skip,
    Play,
    Pause,
    Seek,
    VolumeChange,
    QueueAdd,
    QueueRemove,
    Like,
    Dislike,
    TemporaryOverride,
}
```

**[MUC-EVT-020]** - `user_id`: The persistent, unique identifier (UUID) for each user, as defined in [User Identity and Authentication](user_identity.md). This allows components to reliably identify which user performed an action.

## Edge Case Specifications

### 1. Skip Throttling

**[MUC-SKIP-010]**
- **Requirement:** When one skip click is received, any other skip clicks received within the next 5 seconds are ignored.
- **Mechanism:**
    1. A central component (e.g., a new `UserActionCoordinator` or within the `QueueManager`) subscribes to `UserAction` events.
    2. When a `UserAction` with `action: UserActionType::Skip` is received, the coordinator records the timestamp.
    3. For the next 5 seconds, any subsequent `UserAction` events with `action: UserActionType::Skip` are ignored.
    4. The first `Skip` action triggers the actual skip logic.

- **Sequence Diagram:**
  ```mermaid
  sequenceDiagram
    participant UserA
    participant UserB
    participant API
    participant Coordinator
    participant Playback

    UserA->>API: POST /api/skip
    API->>Coordinator: UserAction(Skip, userA_id)
    Coordinator->>Playback: Skip()
    Playback-->>Coordinator: Skipped
    Coordinator->>API: OK
    API-->>UserA: OK

    UserB->>API: POST /api/skip (within 5s)
    API->>Coordinator: UserAction(Skip, userB_id)
    Coordinator-->>API: Ignored (throttled)
    API-->>UserB: OK (idempotent)
  ```

### 2. Concurrent Queue Removal

**[MUC-QUE-010]**
- **Requirement:** If two or more "remove passage" commands are received for one passage, the later commands are ignored.
- **Mechanism:**
    1. The `QueueManager` handles the `Remove` command.
    2. When a request to remove a passage is received, the `QueueManager` first checks if the passage is still in the queue.
    3. If the passage exists, it is removed, and a `QueueChanged` event is broadcast.
    4. If a second request to remove the same passage arrives, the `QueueManager` will see that the passage is no longer in the queue and will ignore the request.

- **Sequence Diagram:**
  ```mermaid
  sequenceDiagram
    participant UserA
    participant UserB
    participant API
    participant QueueManager

    UserA->>API: POST /api/remove (passageX)
    API->>QueueManager: Remove(passageX)
    QueueManager->>QueueManager: Remove passageX
    QueueManager-->>API: OK
    API-->>UserA: OK

    UserB->>API: POST /api/remove (passageX)
    API->>QueueManager: Remove(passageX)
    QueueManager->>QueueManager: Check for passageX (not found)
    QueueManager-->>API: OK (idempotent)
    API-->>UserB: OK
  ```

### 3. Concurrent Lyric Editing

**[MUC-LYR-010]**
- **Requirement:** Whenever a new lyric text is submitted, it is recorded, overwriting previously submitted lyric texts ("last write wins").
- **Mechanism:**
    1. The `PUT /api/lyrics/:passage_id` endpoint receives the request.
    2. The handler updates the `lyrics` field in the `passages` table for the given `passage_id`.
    3. There is no locking. The last `UPDATE` query to execute will be the one that persists.

- **Sequence Diagram:**
  ```mermaid
  sequenceDiagram
    participant UserA
    participant UserB
    participant API
    participant Database

    UserA->>API: PUT /api/lyrics/passageX (lyricsA)
    API->>Database: UPDATE passages SET lyrics='lyricsA' WHERE ...
    Database-->>API: OK
    API-->>UserA: OK

    UserB->>API: PUT /api/lyrics/passageX (lyricsB)
    API->>Database: UPDATE passages SET lyrics='lyricsB' WHERE ...
    Database-->>API: OK
    API-->>UserB: OK
  ```

---
End of document - McRhythm Multi-User Coordination
