# Whoz Browser: Google Chrome Sync Protocol Specification

> Detailed implementation specification for replicating Google Chrome's built-in Sync protocol in pure Rust (`whoz-sync`).

---

## 1. Overview & Authentication Flow

Google Chrome synchronizes data (Bookmarks, Passwords, Autofill, History, Open Tabs, Extensions, Preferences) with Google Cloud using a dedicated Protobuf-over-HTTPS protocol.

### Authentication Step
1. **OAuth2 Scopes Required:**
   - `https://www.googleapis.com/auth/chromesync`
   - `email`, `profile`
2. **Token Refresh:**
   - Client requests authorization via Google OAuth2 Device Flow or Chromium Token Broker (`https://oauth2.googleapis.com/token`).
   - Token is refreshed asynchronously 60 seconds before expiration.

---

## 2. Server Endpoints & Wire Protocol

- **Primary Command URL:** `https://clients4.google.com/chrome-sync/command/?client_id=<CLIENT_ID>`
- **HTTP Method:** `POST`
- **Content-Type:** `application/octet-stream` (Protocol Buffers)
- **Authorization Header:** `Bearer <OAUTH2_ACCESS_TOKEN>`

### Root Message Structure (`sync.proto`)

```protobuf
syntax = "proto2";
package sync_pb;

message ClientToServerMessage {
  required string share = 1;
  optional int32 protocol_version = 2 [default = 26];
  
  // Commands
  optional CommitMessage commit = 4;
  optional GetUpdatesCallerInfo get_updates = 5;
  optional AuthenticateMessage authenticate = 6;
  optional ClearServerDataMessage clear_server_data = 7;
}

message ClientToServerResponse {
  enum ErrorType {
    SUCCESS = 0;
    ACCESS_DENIED = 1;
    NOT_MY_BIRTHDAY = 2;
    THROTTLED = 3;
    CLEAR_PENDING = 4;
    TRANSIENT_ERROR = 5;
    MIGRATION_DONE = 6;
    DISABLED_BY_ADMIN = 7;
  }
  required ErrorType error_code = 1;
  optional string error_message = 2;
  optional CommitResponse commit = 4;
  optional GetUpdateResponse get_updates = 5;
}
```

---

## 3. Data Specifics (Entities)

Each item synchronized belongs to a specific DataType:

1. **Bookmarks (`BookmarkSpecifics`):**
   - Fields: `url`, `title`, `favicon`, `creation_time_us`, `icon_url`, `meta_info`.
   - Hierarchy: Managed by `parent_id` and unique `server_defined_unique_tag`.
2. **Passwords (`PasswordSpecificsData`):**
   - Fields: `scheme`, `signon_realm`, `origin`, `action`, `username_element`, `username_value`, `password_element`, `password_value`.
   - **Mandatory Nigori Encryption.**
3. **Sessions / Tabs (`SessionSpecifics`):**
   - Fields: `session_tag`, `header`, `tab` (`tab_id`, `window_id`, `navigation_history`).
4. **Extensions (`ExtensionSpecifics`):**
   - Fields: `id`, `version`, `update_url`, `enabled`, `incognito_enabled`.

---

## 4. Nigori End-to-End Cryptography Engine

Google Sync uses the **Nigori Protocol** to protect sensitive user credentials (passwords, autofill credit cards, custom passphrase encryptions):

```
+-------------------------------------------------------------+
|                      USER PASSPHRASE                        |
+-------------------------------------------------------------+
                              |
                              | scrypt / PBKDF2 (salt, N=8192, r=8, p=11)
                              v
+-------------------------------------------------------------+
|                   32-BYTE DERIVED MASTER KEY                |
+-------------------------------------------------------------+
                              |
            +-----------------+-----------------+
            |                                   |
            v                                   v
+-----------------------+           +-----------------------+
|  User-Facing Data     |           |   Encrypted Blob      |
|  Plaintext Payload    |           |   AES-256-GCM Nonce   |
+-----------------------+           +-----------------------+
            |                                   ^
            +------- AES-256-GCM Encrypt -------+
```

### Encryption Key Format
- The encrypted blob is stored as `EncryptedData`:
  ```protobuf
  message EncryptedData {
    optional string key_name = 1;
    optional bytes blob = 2; // IV (12 bytes) + Ciphertext + Auth Tag (16 bytes)
  }
  ```
- In `whoz-sync::nigori`, this is handled by Rust's hardware-accelerated `aes_gcm::Aes256Gcm`.

---

## 5. Local Storage & Conflict Resolution

- Entities are cached locally in SQLite / LevelDB.
- Resolution Strategy: **Server-wins on version mismatch**, with local pending changes merged into the next commit batch.
