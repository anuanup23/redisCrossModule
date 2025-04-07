# Redis Custom Hashmap Module

A Redis module in Rust that implements a custom hashmap storage mechanism. Keys and values are stored in an in-memory hashmap that is not accessible via standard Redis GET commands.

## Features

- Custom key-value storage separate from Redis's main keyspace
- Thread-safe implementation using read-write locks
- Custom commands for accessing and manipulating data

## Commands

- `CUSTOM.SET key value` - Store a key-value pair in the custom hashmap
- `CUSTOM.GET key` - Retrieve a value from the custom hashmap
- `CUSTOM.KEYS` - List all keys in the custom hashmap
- `CUSTOM.DEL key` - Delete a key from the custom hashmap

## Building

```
cargo build --release
```

The compiled module will be in `target/release/libredis_custom_hashmap.so` (Linux/macOS) or `target/release/redis_custom_hashmap.dll` (Windows).

## Loading the Module in Redis

Start Redis with the module:

```
redis-server --loadmodule /path/to/libredis_custom_hashmap.so
```

Or dynamically load the module:

```
MODULE LOAD /path/to/libredis_custom_hashmap.so
```

## Usage Examples

```
> CUSTOM.SET mykey "Hello, Redis modules!"
OK

> CUSTOM.GET mykey
"Hello, Redis modules!"

> GET mykey
(nil)  # Standard Redis GET doesn't access our custom hashmap

> CUSTOM.KEYS
1) "mykey"

> CUSTOM.DEL mykey
(integer) 1

> CUSTOM.GET mykey
(nil)
```

## Notes

- Values stored in this custom hashmap are isolated from Redis's normal key space
- This module is intended as a demonstration of Redis modules in Rust
- The custom hashmap persists only as long as the Redis server is running 