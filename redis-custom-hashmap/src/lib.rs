use std::collections::HashMap;
use std::sync::RwLock;
use redis_module::{
    Context, NextArg, RedisError, RedisResult, RedisString, RedisValue,
};

// Global hashmap to store our key-value pairs
static mut CUSTOM_HASHMAP: Option<RwLock<HashMap<String, String>>> = None;

// Initialize the hashmap
pub fn init_hashmap() -> &'static RwLock<HashMap<String, String>> {
    unsafe {
        if CUSTOM_HASHMAP.is_none() {
            CUSTOM_HASHMAP = Some(RwLock::new(HashMap::new()));
        }
        CUSTOM_HASHMAP.as_ref().unwrap()
    }
}

// Public API functions for other modules to use directly
#[no_mangle]
pub extern "C" fn custom_hashmap_set(key: *const libc::c_char, value: *const libc::c_char) -> libc::c_int {
    if key.is_null() || value.is_null() {
        return 0;
    }
    
    let key_str = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().to_string() };
    let value_str = unsafe { std::ffi::CStr::from_ptr(value).to_string_lossy().to_string() };
    
    let hashmap = init_hashmap();
    match hashmap.write() {
        Ok(mut map) => {
            map.insert(key_str, value_str);
            1
        },
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn custom_hashmap_get(key: *const libc::c_char) -> *mut libc::c_char {
    if key.is_null() {
        return std::ptr::null_mut();
    }
    
    let key_str = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().to_string() };
    
    let hashmap = init_hashmap();
    match hashmap.read() {
        Ok(map) => {
            match map.get(&key_str) {
                Some(value) => {
                    let c_str = std::ffi::CString::new(value.clone()).unwrap();
                    c_str.into_raw()
                },
                None => std::ptr::null_mut(),
            }
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn custom_hashmap_del(key: *const libc::c_char) -> libc::c_int {
    if key.is_null() {
        return 0;
    }
    
    let key_str = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().to_string() };
    
    let hashmap = init_hashmap();
    match hashmap.write() {
        Ok(mut map) => {
            if map.remove(&key_str).is_some() { 1 } else { 0 }
        },
        Err(_) => 0,
    }
}

// Custom command to set a key-value pair
fn custom_set(_ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    let value = args.next_string()?;
    
    let hashmap = init_hashmap();
    let mut map = hashmap.write().map_err(|_| {
        RedisError::String("Failed to acquire write lock".to_string())
    })?;
    
    map.insert(key, value);
    
    Ok(RedisValue::SimpleStringStatic("OK"))
}

// Custom command to get a value by key
fn custom_get(_ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    
    let hashmap = init_hashmap();
    let map = hashmap.read().map_err(|_| {
        RedisError::String("Failed to acquire read lock".to_string())
    })?;
    
    match map.get(&key) {
        Some(value) => Ok(RedisValue::BulkString(value.clone().into())),
        None => Ok(RedisValue::Null),
    }
}

// List all keys in the custom hashmap
fn custom_keys(_ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() != 1 {
        return Err(RedisError::WrongArity);
    }
    
    let hashmap = init_hashmap();
    let map = hashmap.read().map_err(|_| {
        RedisError::String("Failed to acquire read lock".to_string())
    })?;
    
    let keys: Vec<RedisValue> = map.keys()
        .map(|k| RedisValue::BulkString(k.clone().into()))
        .collect();
    
    Ok(RedisValue::Array(keys))
}

// Delete a key from the custom hashmap
fn custom_del(_ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    
    let hashmap = init_hashmap();
    let mut map = hashmap.write().map_err(|_| {
        RedisError::String("Failed to acquire write lock".to_string())
    })?;
    
    let removed = map.remove(&key).is_some();
    
    Ok(RedisValue::Integer(if removed { 1 } else { 0 }))
}

// Redis module initialization with the correct format for v2.0.7
redis_module::redis_module! {
    name: "custom_hashmap",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["custom.set", custom_set, "write", 1, 1, 1],
        ["custom.get", custom_get, "readonly", 1, 1, 1],
        ["custom.keys", custom_keys, "readonly", 0, 0, 0],
        ["custom.del", custom_del, "write", 1, 1, 1],
    ],
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
