# Redis Modules for Custom Storage and Session Management


Test Line
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

The project demonstrates two different approaches for integration between Redis modules:

### 1. Using Redis Commands (Original Approach)

The session manager module communicates with the custom hashmap module by executing Redis commands through the Redis command interface. When a session is created or deleted, the session manager calls the appropriate custom hashmap commands to manage the mapping between user keys and session IDs.

### 2. Direct Module Communication (Enhanced Approach)

The custom hashmap module exposes its storage through a C-compatible FFI (Foreign Function Interface), allowing other modules to directly access its functionality without going through the Redis command layer. The session manager module can load and call these functions directly, improving performance by eliminating the Redis command overhead.

The direct communication is implemented using:
- Exported C functions with the `#[no_mangle]` attribute from the custom hashmap module
- Dynamic loading of these functions in the session manager using the `libloading` crate
- A fallback mechanism that uses Redis commands if direct loading fails

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