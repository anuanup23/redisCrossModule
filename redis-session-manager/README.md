# Redis Session Manager Module

A Redis module in Rust that implements session management functionality by integrating with the `custom_hashmap` module. This module provides session creation, retrieval, and management with support for storing arbitrary key-value data within sessions.

## Features

- Session creation with automatic UUID generation
- Integration with the custom_hashmap module for key validation
- Session data storage
- Listing active sessions
- Session deletion

## Prerequisites

This module depends on the `custom_hashmap` module being loaded first, as it uses the custom hashmap for key validation.

## Building

```
cargo build --release
```

The compiled module will be in `target/release/libredis_session_manager.so` (Linux) or `target/release/libredis_session_manager.dylib` (macOS).

## Loading the Module in Redis

Start Redis with both modules:

```
redis-server --loadmodule /path/to/libredis_custom_hashmap.dylib --loadmodule /path/to/libredis_session_manager.dylib
```

## Commands

### Session Management

- `SESSION.CREATE key` - Create a new session associated with a key. If the key already exists in the custom hashmap, it returns the existing session.
- `SESSION.GET session_id` - Retrieve full information about a session by its ID.
- `SESSION.LIST` - List all active sessions.
- `SESSION.DELETE session_id` - Delete a session by ID (also removes the key from the custom hashmap).

### Session Data

- `SESSION.ADD_DATA session_id key value` - Add or update a key-value pair in the session.
- `SESSION.GET_DATA session_id key` - Retrieve a value for a specific key from the session.

## Usage Example

```
> SESSION.CREATE user123
"Session created: 8f0f964d-1e9b-4f25-9567-0b9b5d32a7c1"

> SESSION.LIST
1) "ID: 8f0f964d-1e9b-4f25-9567-0b9b5d32a7c1, Key: user123, Created: 2025-04-07T14:40:00Z"

> SESSION.ADD_DATA 8f0f964d-1e9b-4f25-9567-0b9b5d32a7c1 username "John Doe"
OK

> SESSION.GET_DATA 8f0f964d-1e9b-4f25-9567-0b9b5d32a7c1 username
"John Doe"

> SESSION.GET 8f0f964d-1e9b-4f25-9567-0b9b5d32a7c1
"{\"id\":\"8f0f964d-1e9b-4f25-9567-0b9b5d32a7c1\",\"user_key\":\"user123\",\"created_at\":\"2025-04-07T14:40:00Z\",\"last_accessed\":\"2025-04-07T14:42:30Z\",\"data\":{\"username\":\"John Doe\"}}"

> CUSTOM.GET user123
"8f0f964d-1e9b-4f25-9567-0b9b5d32a7c1"

> SESSION.DELETE 8f0f964d-1e9b-4f25-9567-0b9b5d32a7c1
(integer) 1

> CUSTOM.GET user123
(nil)
```

## Notes

- Each session has a unique ID (UUID)
- Sessions store creation and last accessed timestamps
- Sessions maintain their own key-value store for arbitrary data
- The module requires the custom_hashmap module to be loaded first
- The custom_hashmap module is used to validate keys and maintain the association between user keys and session IDs 