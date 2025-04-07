# Redis Modules for Custom Storage and Session Management

This project demonstrates the power of extending Redis with custom modules written in Rust. It consists of two main components:

## 1. Custom Hashmap Module

A Redis module that provides a custom hashmap implementation with its own namespace, separate from standard Redis keys. This module allows:

- Setting values in the custom hashmap
- Getting values from the custom hashmap
- Deleting values from the custom hashmap
- Checking if a key exists in the custom hashmap

### Commands

- `CUSTOM.SET key value` - Set a key-value pair in the custom hashmap
- `CUSTOM.GET key` - Get a value from the custom hashmap
- `CUSTOM.DEL key` - Delete a key from the custom hashmap
- `CUSTOM.EXISTS key` - Check if a key exists in the custom hashmap

## 2. Session Manager Module

A Redis module that provides session management functionality, integrated with the custom hashmap module. This module allows:

- Creating sessions with unique IDs
- Retrieving session information
- Listing all active sessions
- Adding data to sessions
- Retrieving data from sessions
- Deleting sessions

### Commands

- `SESSION.CREATE user_key` - Create a new session for a user
- `SESSION.GET session_id` - Get session details
- `SESSION.LIST` - List all active sessions
- `SESSION.ADD_DATA session_id key value` - Add data to a session
- `SESSION.GET_DATA session_id key` - Get data from a session
- `SESSION.DELETE session_id` - Delete a session

## Integration

The session manager module communicates with the custom hashmap module to store user keys and their associated session IDs. When a session is deleted, the corresponding entry in the custom hashmap is also removed.

## Building and Running

Each module has its own build process using Cargo:

```bash
# Build custom hashmap module
cd redis-custom-hashmap
cargo build --release

# Build session manager module
cd ../redis-session-manager
cargo build --release
```

To run Redis with both modules:

```bash
redis-server --loadmodule ./redis-custom-hashmap/target/release/libredis_custom_hashmap.dylib --loadmodule ./redis-session-manager/target/release/libredis_session_manager.dylib
```

## Example Usage

```bash
# Create a session
redis-cli SESSION.CREATE user123
# Output: Session created: 9d93a2f5-e560-451e-b21b-e51b10e63b14

# Check custom hashmap
redis-cli CUSTOM.GET user123
# Output: "9d93a2f5-e560-451e-b21b-e51b10e63b14"

# Add data to session
redis-cli SESSION.ADD_DATA 9d93a2f5-e560-451e-b21b-e51b10e63b14 email "user@example.com"
# Output: OK

# Get data from session
redis-cli SESSION.GET_DATA 9d93a2f5-e560-451e-b21b-e51b10e63b14 email
# Output: "user@example.com"

# Delete session
redis-cli SESSION.DELETE 9d93a2f5-e560-451e-b21b-e51b10e63b14
# Output: (integer) 1

# Verify deletion from custom hashmap
redis-cli CUSTOM.GET user123
# Output: (nil)
``` 