# User Identity and Authentication

**🤝 TIER 2 - DESIGN SPECIFICATION**

Defines user identity, authentication, and account management. See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [API Design](api_design.md) | [Database Schema](database_schema.md)

---

## Overview

This document specifies the design for user identity to support persistent, long-term musical taste profiles and multi-user concurrent access.

**Key Design Principles:**
- **Universal Identity Requirement**: All users must have a UUID (either Anonymous or personal account)
- **Flexible Authentication**: Users choose between no-password Anonymous access or personal accounts
- **Persistent Sessions**: Browser-based session storage eliminates repeated login prompts
- **Multi-User Support**: Multiple users (anonymous and authenticated) can use the system simultaneously
- **Taste Profile Isolation**: Each user UUID has independent taste data (likes, dislikes, preferences)

User identity is required for:
- Recording likes and dislikes (Phase 1: Full and Lite versions)
- Building user-specific musical taste profiles (Phase 1/2)
- Tracking user-specific play history (Phase 2)
- Future ListenBrainz integration (Phase 2)

## 1. User Identity

**[UID-ID-010]** The primary identifier for a user within the system shall be a UUID (Universally Unique Identifier).

**[UID-ID-020]** All records in the database that are associated with a user (e.g., `likes_dislikes`) shall use the user's UUID as a foreign key.

### 1.1. Client-Side Storage

**[UID-STOR-010]** Once a user has established a UUID with the server (either as an Anonymous user or by logging in), the UUID shall be stored on the user's client (e.g., in the browser's `localStorage`).

**[UID-STOR-020]** The stored UUID shall have a rolling expiration of one year. The expiration date should be reset to one year from the current date upon each successful connection to the server.

## 2. Initial Connection Flow

## Authentication Modes
<a name="authentication-modes"></a>

**[UID-FLOW-010]** When a user connects to the server without a stored UUID (first visit or after one-year expiration), the user interface shall present them with three choices:
1.  **Proceed Anonymously** - Use shared Anonymous account, no password required
2.  **Create a new account** - Register unique username/password, get personal UUID
3.  **Login to an existing account** - Authenticate with existing credentials, retrieve UUID

**[UID-FLOW-020]** Once authenticated by any method, the browser stores the UUID in localStorage with a rolling one-year expiration. Subsequent visits within the one-year window automatically use the stored UUID without prompting.

**[UID-FLOW-030]** On each successful connection, the one-year expiration timer resets, ensuring active users maintain persistent sessions indefinitely.

## 3. User Types

### 3.1. Anonymous User

**[UID-ANON-010]** A single, shared "Anonymous" user shall exist in the system with a predefined, static UUID.
- **Username:** `Anonymous`
- **Authentication:** No password is required to use the Anonymous account.

**[UID-ANON-020]** When a user chooses to "Proceed Anonymously," the server shall provide the client with the static UUID for the Anonymous user. This UUID is then stored on the client as per the client-side storage rules.

**[UID-ANON-030]** All users accessing the system as Anonymous share:
- The same UUID
- The same likes and dislikes data
- The same taste profile
- The same play history (if tracked)

**[UID-ANON-040]** Anonymous mode is suitable for:
- Casual users who don't want account management
- Public installations where personal profiles are not needed
- Initial system evaluation before creating an account

**[UID-ANON-050]** Anonymous users may convert to registered users at any time. Upon conversion:
- A new unique UUID is generated for the registered account
- Previous Anonymous session data is NOT transferred to the new account
- The user starts with a fresh taste profile

### 3.2. Registered User

**[UID-REG-010]** A registered user is an identity with a unique username, a password, and a unique, persistent UUID.

## 4. Account Creation

**[UID-CREATE-010]** When a user chooses to "Create a new account," they shall be prompted to provide a username and a password.

### 4.1. Username Requirements
- **[UID-USER-010]** **Uniqueness:** The username must not already exist in the system.
- **[UID-USER-020]** **Characters:** All characters must be valid UTF-8. Invisible characters (e.g., zero-width spaces) are not permitted.
- **[UID-USER-030]** **Length:** The username must be between 1 and 63 characters, inclusive (i.e., `length < 64`).

### 4.2. Password Requirements
- **[UID-PASS-010]** **Characters:** All characters must be valid UTF-8. Invisible characters are not permitted.
- **[UID-PASS-020]** **Length:** The password must be between 1 and 63 characters, inclusive (i.e., `length < 64`).

### 4.3. Password Transmission Protection

**[UID-PASS-030]** To protect passwords during transmission over insecure HTTP connections, McRhythm uses **client-side password hashing** before transmission.

**[UID-PASS-031]** Client-side password hashing protocol:

1. **Client requests account creation:**
   - `POST /api/auth/request-create`
   - Request body: `{"username": "alice"}`
   - Server validates username availability and UTF-8 compliance

2. **Server responds with challenge:**
   - Response: `{"status": "ok", "challenge": "random-32-byte-hex-string"}`
   - Challenge is a cryptographically secure random value (256 bits, represented as 64 hex characters)
   - Challenge is single-use and expires after 60 seconds

3. **Client hashes password with challenge:**
   - Compute: `client_hash = SHA-256(challenge || password)`
   - Where `||` denotes concatenation
   - Send: `POST /api/auth/create`
   - Request body: `{"username": "alice", "challenge": "...", "client_hash": "..."}`

4. **Server creates account:**
   - Validates challenge (not expired, matches username)
   - Generates UUID, salt
   - Computes: `stored_hash = Argon2id(salt || UUID || client_hash)`
   - Stores: username, UUID, salt, stored_hash
   - Response: `{"status": "ok", "uuid": "..."}`

**[UID-PASS-032]** Security properties:
- Password never transmitted in cleartext over HTTP
- Challenge prevents replay attacks (single-use, time-limited)
- Server never sees actual password, only `client_hash`
- Stored hash uses Argon2id with salt and UUID for additional security

**[UID-PASS-033]** Challenge expiration and cleanup:
- Challenges expire 60 seconds after generation
- Server maintains in-memory challenge store (map: username → {challenge, timestamp})
- Expired challenges automatically removed on next cleanup cycle (every 5 minutes)
- Maximum 1000 pending challenges (DOS prevention)

### 4.4. Account Record Creation

**[UID-CREATE-020]** Upon successful completion of the password transmission protocol (4.3), the system shall:
1.  Generate a new, unique UUID for the user.
2.  Create a new user record in the database containing the UUID and username.
3.  Generate a new, cryptographically secure random salt (256 bits).
4.  Compute: `stored_hash = Argon2id(salt || UUID || client_hash)`
5.  Store: username, UUID, salt, stored_hash in the database.
6.  Return the UUID to the client for localStorage storage.

**[UID-CREATE-030]** Argon2id parameters:
- **Memory cost:** 64 MB (65536 KiB)
- **Time cost:** 3 iterations
- **Parallelism:** 1 thread
- **Salt length:** 32 bytes (256 bits)
- **Hash output length:** 32 bytes (256 bits)

## 5. Authentication (Login)

**[UID-AUTH-010]** When a user chooses to "Login to an existing account," they shall be prompted for their username and password.

**[UID-AUTH-020]** Login uses the same challenge-response protocol as account creation to protect passwords during transmission.

**[UID-AUTH-021]** Client-side password hashing protocol for login:

1. **Client requests login challenge:**
   - `POST /api/auth/request-login`
   - Request body: `{"username": "alice"}`
   - Server validates username exists

2. **Server responds with challenge:**
   - Response: `{"status": "ok", "challenge": "random-32-byte-hex-string"}`
   - Challenge is cryptographically secure random value (256 bits)
   - Challenge is single-use and expires after 60 seconds

3. **Client hashes password with challenge:**
   - Compute: `client_hash = SHA-256(challenge || password)`
   - Send: `POST /api/auth/login`
   - Request body: `{"username": "alice", "challenge": "...", "client_hash": "..."}`

4. **Server authenticates:**
   - Validates challenge (not expired, matches username)
   - Retrieves user record: UUID, salt, stored_hash
   - Computes: `verification_hash = Argon2id(salt || UUID || client_hash)`
   - Compares `verification_hash` with `stored_hash`
   - If match: Response `{"status": "ok", "uuid": "..."}`
   - If mismatch: Response `{"status": "error", "message": "Invalid credentials"}`

**[UID-AUTH-030]** Security properties (same as account creation):
- Password never transmitted in cleartext
- Challenge prevents replay attacks
- Rate limiting: Maximum 5 failed login attempts per username per 15 minutes
- Failed attempts logged with timestamp and source IP

## 6. Account Management

**[UID-MGMT-010]** Once logged in, a registered user shall have the option to change their username or password. The user's UUID cannot be changed.

**[UID-MGMT-020]** To change a password, the user must provide their current password and new password. Uses challenge-response protocol for both.

**[UID-MGMT-021]** Password change protocol:

1. **Client requests password change challenge:**
   - `POST /api/auth/request-password-change`
   - Request body: `{"username": "alice"}`
   - Must include session authentication (UUID in header or cookie)

2. **Server responds with challenge:**
   - Validates user is authenticated (session UUID matches username)
   - Response: `{"status": "ok", "challenge": "random-32-byte-hex-string"}`

3. **Client hashes both passwords:**
   - Compute: `current_hash = SHA-256(challenge || current_password)`
   - Compute: `new_hash = SHA-256(challenge || new_password)`
   - Send: `POST /api/auth/change-password`
   - Request body: `{"username": "alice", "challenge": "...", "current_hash": "...", "new_hash": "..."}`

4. **Server updates password:**
   - Validates challenge
   - Verifies `current_hash` against stored hash (same as login)
   - If verification succeeds:
     - Generates new salt
     - Computes: `new_stored_hash = Argon2id(new_salt || UUID || new_hash)`
     - Updates database with new salt and new stored hash
     - Response: `{"status": "ok"}`
   - If verification fails: Response `{"status": "error", "message": "Current password incorrect"}`

**[UID-MGMT-030]** Username changes do not require password transmission and use simple authenticated API call.

## 7. Concurrent Sessions

**[UID-SESS-010]** A single user account (identified by its UUID) may be authenticated from multiple clients or browsers simultaneously. The server will not invalidate previous sessions upon a new login. All authenticated clients for all users will receive the same real-time event stream as described in [event_system.md](event_system.md).

### 8.1. Account Maintenance Tool

**[UID-RESET-040]** The password reset tool will be implemented as a standalone command-line program named `wkmp-account-maintenance`.

**[UID-RESET-050]** If the program is executed with no arguments, invalid arguments, or the `--help` flag, it will display a help message outlining the available commands and their syntax.

#### **Usage**

```
wkmp-account-maintenance <command> [options]

Commands:
  --list                  List all registered usernames and their UUIDs.

  --reset                 Reset a user's password.
                          Requires --username and --password options.

  --help                  Display this help message.

Options for --reset:
  --username <username>   The username for the account to modify.
  --password <password>   The new password to set for the user.
```

#### **Functions**

*   **List Users:**
    *   **Command:** `wkmp-account-maintenance --list`
    *   **Action:** Retrieves and displays a table of all registered usernames and their corresponding UUIDs, allowing the administrator to identify the correct user.

*   **Reset Password:**
    *   **Command:** `wkmp-account-maintenance --reset --username <username> --password <new-password>`
    *   **Action:** Finds the user by their username and updates their account with a new password. It computes and stores the new salted hash according to the defined authentication mechanism.
    *   **Output:** A confirmation message on success or an error message if the user is not found.

----
End of document - User Identity and Authentication
