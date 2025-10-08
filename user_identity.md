# User Identity and Authentication

**🤝 TIER 2 - DESIGN SPECIFICATION**

Defines user identity, authentication, and account management. See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [API Design](api_design.md) | [Database Schema](database_schema.md)

---

## Overview

This document specifies the design for user identity to support persistent, long-term musical taste profiles. While the system does not require authentication for core playback functionality, a persistent user identity is necessary for features like Likes/Dislikes. The design prioritizes ease of use while providing basic account management features.

## 1. User Identity

**[UID-ID-010]** The primary identifier for a user within the system shall be a UUID (Universally Unique Identifier).

**[UID-ID-020]** All records in the database that are associated with a user (e.g., `likes_dislikes`) shall use the user's UUID as a foreign key.

### 1.1. Client-Side Storage

**[UID-STOR-010]** Once a user has established a UUID with the server (either as an Anonymous user or by logging in), the UUID shall be stored on the user's client (e.g., in the browser's `localStorage`).

**[UID-STOR-020]** The stored UUID shall have a rolling expiration of one year. The expiration date should be reset to one year from the current date upon each successful connection to the server.

## 2. Initial Connection Flow

**[UID-FLOW-010]** When a user connects to the server without a stored UUID, the user interface shall present them with three choices:
1.  **Proceed Anonymously**
2.  **Create a new account**
3.  **Login to an existing account**

## 3. User Types

### 3.1. Anonymous User

**[UID-ANON-010]** A single, shared "Anonymous" user shall exist in the system with a predefined, static UUID.
- **Username:** `Anonymous`
- **Authentication:** No password is required to use the Anonymous account.

**[UID-ANON-020]** When a user chooses to "Proceed Anonymously," the server shall provide the client with the static UUID for the Anonymous user. This UUID is then stored on the client as per the client-side storage rules.

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

### 4.3. Account Record Creation

**[UID-CREATE-020]** Upon submission of a valid and unique username and password, the system shall:
1.  Generate a new, unique UUID for the user.
2.  Create a new user record in the database containing the UUID and username.
3.  Generate a new, cryptographically secure random salt.
4.  Generate a secure hash of the random salt concatenated with the user's UUID and their password.
5.  Store both the salt and the hash in the database for future authentication.

## 5. Authentication (Login)

**[UID-AUTH-010]** When a user chooses to "Login to an existing account," they shall be prompted for their username and password.

**[UID-AUTH-020]** To authenticate, the system will:
1.  Retrieve the user's record from the database based on the provided username.
2.  Retrieve the user's UUID, the stored salt, and the stored hash.
3.  Re-create the hash using the stored salt, the retrieved UUID, and the user-provided password.
4.  If the generated hash matches the stored hash, authentication is successful. The server then provides the client with the user's UUID to be stored locally.

## 6. Account Management

**[UID-MGMT-010]** Once logged in, a registered user shall have the option to change their username or password. The user's UUID cannot be changed.

**[UID-MGMT-020]** To change a username or password, the user must provide their current, valid password along with the requested change. The system must re-authenticate the user with the current password before applying any changes.

## 7. Concurrent Sessions

**[UID-SESS-010]** A single user account (identified by its UUID) may be authenticated from multiple clients or browsers simultaneously. The server will not invalidate previous sessions upon a new login. All authenticated clients for all users will receive the same real-time event stream as described in [event_system.md](event_system.md).

### 8.1. Account Maintenance Tool

**[UID-RESET-040]** The password reset tool will be implemented as a standalone command-line program named `mcrhythm-account-maintenance`.

**[UID-RESET-050]** If the program is executed with no arguments, invalid arguments, or the `--help` flag, it will display a help message outlining the available commands and their syntax.

#### **Usage**

```
mcrhythm-account-maintenance <command> [options]

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
    *   **Command:** `mcrhythm-account-maintenance --list`
    *   **Action:** Retrieves and displays a table of all registered usernames and their corresponding UUIDs, allowing the administrator to identify the correct user.

*   **Reset Password:**
    *   **Command:** `mcrhythm-account-maintenance --reset --username <username> --password <new-password>`
    *   **Action:** Finds the user by their username and updates their account with a new password. It computes and stores the new salted hash according to the defined authentication mechanism.
    *   **Output:** A confirmation message on success or an error message if the user is not found.

----
End of document - User Identity and Authentication
