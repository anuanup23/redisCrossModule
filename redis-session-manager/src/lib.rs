use std::collections::HashMap;
use std::sync::RwLock;
use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

// Dynamic loading approach using libloading
use libloading::{Library, Symbol};

// Type aliases for our function signatures
type SetFn = unsafe extern "C" fn(*const c_char, *const c_char) -> libc::c_int;
type GetFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type DelFn = unsafe extern "C" fn(*const c_char) -> libc::c_int;

// Global variables to store our dynamically loaded functions
static mut SET_FN: Option<Symbol<'static, SetFn>> = None;
static mut GET_FN: Option<Symbol<'static, GetFn>> = None;
static mut DEL_FN: Option<Symbol<'static, DelFn>> = None;
static mut LIB_HANDLE: Option<Library> = None;

// Initialize and load the custom hashmap library
fn init_custom_hashmap_lib() -> Result<(), RedisError> {
    unsafe {
        if LIB_HANDLE.is_none() {
            // Try to load the library
            let lib = match Library::new("libredis_custom_hashmap.dylib") {
                Ok(lib) => lib,
                Err(e) => {
                    // If we can't load the library, we'll fall back to Redis commands
                    return Err(RedisError::String(format!("Failed to load custom hashmap library: {}", e)));
                }
            };
            
            // Get the symbols
            let set_fn = match lib.get::<SetFn>(b"custom_hashmap_set") {
                Ok(sym) => sym,
                Err(e) => return Err(RedisError::String(format!("Failed to load custom_hashmap_set: {}", e))),
            };
                
            let get_fn = match lib.get::<GetFn>(b"custom_hashmap_get") {
                Ok(sym) => sym,
                Err(e) => return Err(RedisError::String(format!("Failed to load custom_hashmap_get: {}", e))),
            };
                
            let del_fn = match lib.get::<DelFn>(b"custom_hashmap_del") {
                Ok(sym) => sym,
                Err(e) => return Err(RedisError::String(format!("Failed to load custom_hashmap_del: {}", e))),
            };
                
            // Need to use transmute for static lifetime, as these will live for the entire program
            SET_FN = Some(std::mem::transmute(set_fn));
            GET_FN = Some(std::mem::transmute(get_fn));
            DEL_FN = Some(std::mem::transmute(del_fn));
            
            // Now we can store the library
            LIB_HANDLE = Some(lib);
        }
    }
    
    Ok(())
}

// Helper function to get a value from the custom hashmap
fn custom_get(key: &str) -> Option<String> {
    // Try to initialize the custom hashmap library
    if let Err(_) = init_custom_hashmap_lib() {
        return None;
    }
    
    unsafe {
        let get_fn = match GET_FN.as_ref() {
            Some(f) => f,
            None => return None,
        };
        
        let key_cstr = match CString::new(key) {
            Ok(cstr) => cstr,
            Err(_) => return None,
        };
        
        let value_ptr = get_fn(key_cstr.as_ptr());
        if value_ptr.is_null() {
            return None;
        }
        
        let value_cstr = CStr::from_ptr(value_ptr);
        let result = value_cstr.to_string_lossy().to_string();
        
        // Need to free the memory allocated by custom_hashmap_get
        libc::free(value_ptr as *mut libc::c_void);
        
        Some(result)
    }
}

// Helper function to set a value in the custom hashmap
fn custom_set(key: &str, value: &str) -> bool {
    // Try to initialize the custom hashmap library
    if let Err(_) = init_custom_hashmap_lib() {
        return false;
    }
    
    unsafe {
        let set_fn = match SET_FN.as_ref() {
            Some(f) => f,
            None => return false,
        };
        
        let key_cstr = match CString::new(key) {
            Ok(cstr) => cstr,
            Err(_) => return false,
        };
        
        let value_cstr = match CString::new(value) {
            Ok(cstr) => cstr,
            Err(_) => return false,
        };
        
        let result = set_fn(key_cstr.as_ptr(), value_cstr.as_ptr());
        
        result == 1
    }
}

// Helper function to delete a key from the custom hashmap
fn custom_del(key: &str) -> bool {
    // Try to initialize the custom hashmap library
    if let Err(_) = init_custom_hashmap_lib() {
        return false;
    }
    
    unsafe {
        let del_fn = match DEL_FN.as_ref() {
            Some(f) => f,
            None => return false,
        };
        
        let key_cstr = match CString::new(key) {
            Ok(cstr) => cstr,
            Err(_) => return false,
        };
        
        let result = del_fn(key_cstr.as_ptr());
        
        result == 1
    }
}

// Session structure
#[derive(Debug, Serialize, Deserialize)]
struct Session {
    id: String,
    user_key: String,
    created_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    data: HashMap<String, String>,
}

// Global sessions store
static mut SESSIONS: Option<RwLock<HashMap<String, Session>>> = None;

// Initialize the sessions store
fn init_sessions() -> &'static RwLock<HashMap<String, Session>> {
    unsafe {
        if SESSIONS.is_none() {
            SESSIONS = Some(RwLock::new(HashMap::new()));
        }
        SESSIONS.as_ref().unwrap()
    }
}

// Create a new session
fn create_session(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    
    // Try to get the key from custom hashmap directly via FFI
    match custom_get(&key) {
        Some(session_id) => {
            // Check if session exists
            let sessions = init_sessions();
            let mut sessions_map = sessions.write().map_err(|_| {
                RedisError::String("Failed to acquire write lock".to_string())
            })?;
            
            // Update the last accessed time if session exists
            if let Some(session) = sessions_map.get_mut(&session_id) {
                session.last_accessed = Utc::now();
                Ok(RedisValue::SimpleString(format!("Session exists: {}", session_id)))
            } else {
                // Create a new session if session ID exists in hashmap but not in our store
                let session = Session {
                    id: session_id.clone(),
                    user_key: key,
                    created_at: Utc::now(),
                    last_accessed: Utc::now(),
                    data: HashMap::new(),
                };
                
                sessions_map.insert(session_id.clone(), session);
                Ok(RedisValue::SimpleString(format!("Session recreated: {}", session_id)))
            }
        },
        None => {
            // If key doesn't exist, create a new session
            // Generate a new session ID
            let session_id = Uuid::new_v4().to_string();
            
            // Add key to custom hashmap with session_id as value directly
            if !custom_set(&key, &session_id) {
                // Fall back to Redis commands if direct call fails
                match ctx.call("custom.set", &[&key, &session_id]) {
                    Ok(_) => {},
                    Err(err) => {
                        return Err(RedisError::String(format!("Failed to call custom.set: {}", err)));
                    }
                }
            }
            
            // Create a new session object
            let session = Session {
                id: session_id.clone(),
                user_key: key,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                data: HashMap::new(),
            };
            
            // Store the session in our internal sessions store
            let sessions = init_sessions();
            let mut sessions_map = sessions.write().map_err(|_| {
                RedisError::String("Failed to acquire write lock".to_string())
            })?;
            
            sessions_map.insert(session_id.clone(), session);
            
            Ok(RedisValue::SimpleString(format!("Session created: {}", session_id)))
        }
    }
}

// Get session by ID
fn get_session(_ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let session_id = args.next_string()?;
    
    let sessions = init_sessions();
    let sessions_map = sessions.read().map_err(|_| {
        RedisError::String("Failed to acquire read lock".to_string())
    })?;
    
    match sessions_map.get(&session_id) {
        Some(session) => {
            let json = serde_json::to_string(session).map_err(|e| {
                RedisError::String(format!("Failed to serialize session: {}", e))
            })?;
            Ok(RedisValue::BulkString(json.into()))
        },
        None => Ok(RedisValue::Null),
    }
}

// List all sessions
fn list_sessions(_ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() != 1 {
        return Err(RedisError::WrongArity);
    }
    
    let sessions = init_sessions();
    let sessions_map = sessions.read().map_err(|_| {
        RedisError::String("Failed to acquire read lock".to_string())
    })?;
    
    let session_list: Vec<RedisValue> = sessions_map.keys()
        .map(|id| {
            let session = &sessions_map[id];
            let output = format!("ID: {}, Key: {}, Created: {}", 
                session.id, 
                session.user_key,
                session.created_at.to_rfc3339());
            RedisValue::BulkString(output.into())
        })
        .collect();
    
    Ok(RedisValue::Array(session_list))
}

// Add data to a session
fn add_session_data(_ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let session_id = args.next_string()?;
    let data_key = args.next_string()?;
    let data_value = args.next_string()?;
    
    let sessions = init_sessions();
    let mut sessions_map = sessions.write().map_err(|_| {
        RedisError::String("Failed to acquire write lock".to_string())
    })?;
    
    match sessions_map.get_mut(&session_id) {
        Some(session) => {
            session.data.insert(data_key, data_value);
            session.last_accessed = Utc::now();
            Ok(RedisValue::SimpleStringStatic("OK"))
        },
        None => Err(RedisError::String(format!("Session not found: {}", session_id))),
    }
}

// Get data from a session
fn get_session_data(_ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let session_id = args.next_string()?;
    let data_key = args.next_string()?;
    
    let sessions = init_sessions();
    let mut sessions_map = sessions.write().map_err(|_| {
        RedisError::String("Failed to acquire write lock".to_string())
    })?;
    
    match sessions_map.get_mut(&session_id) {
        Some(session) => {
            session.last_accessed = Utc::now();
            match session.data.get(&data_key) {
                Some(value) => Ok(RedisValue::BulkString(value.clone().into())),
                None => Ok(RedisValue::Null),
            }
        },
        None => Err(RedisError::String(format!("Session not found: {}", session_id))),
    }
}

// Delete a session
fn delete_session(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let session_id = args.next_string()?;
    
    let sessions = init_sessions();
    let mut sessions_map = sessions.write().map_err(|_| {
        RedisError::String("Failed to acquire write lock".to_string())
    })?;
    
    if let Some(session) = sessions_map.remove(&session_id) {
        // Try to remove from custom hashmap directly via FFI
        if !custom_del(&session.user_key) {
            // Fall back to Redis commands if direct call fails
            match ctx.call("custom.del", &[&session.user_key]) {
                Ok(_) => {},
                Err(err) => {
                    // Re-add the session since we failed to remove from custom hashmap
                    sessions_map.insert(session_id.clone(), session);
                    return Err(RedisError::String(format!("Failed to call custom.del: {}", err)));
                }
            }
        }
        
        Ok(RedisValue::Integer(1))
    } else {
        Ok(RedisValue::Integer(0))
    }
}

// Redis module initialization
redis_module::redis_module! {
    name: "session_manager",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["session.create", create_session, "write", 1, 1, 1],
        ["session.get", get_session, "readonly", 1, 1, 1],
        ["session.list", list_sessions, "readonly", 0, 0, 0],
        ["session.add_data", add_session_data, "write", 1, 1, 1],
        ["session.get_data", get_session_data, "readonly", 1, 1, 1],
        ["session.delete", delete_session, "write", 1, 1, 1],
    ],
}
