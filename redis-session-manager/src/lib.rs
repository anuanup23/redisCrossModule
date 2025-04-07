use std::collections::HashMap;
use std::sync::RwLock;
use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

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
    
    // Try to get the key using custom.get command
    // First attempt to use the Context call method
    let result = match ctx.call("custom.get", &[&key]) {
        Ok(value) => value,
        Err(err) => {
            // If there's an error calling the command, we'll assume the key doesn't exist
            // This might happen if the module isn't loaded
            return Err(RedisError::String(format!("Failed to call custom.get: {}", err)));
        }
    };
    
    // If key doesn't exist, create a new session
    if let RedisValue::Null = result {
        // Generate a new session ID
        let session_id = Uuid::new_v4().to_string();
        
        // Add key to custom hashmap with session_id as value
        match ctx.call("custom.set", &[&key, &session_id]) {
            Ok(_) => {
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
            },
            Err(err) => {
                Err(RedisError::String(format!("Failed to call custom.set: {}", err)))
            }
        }
    } else if let RedisValue::BulkString(session_id) = result {
        // Check if session exists
        let session_id_str = session_id.to_string();
        let sessions = init_sessions();
        let mut sessions_map = sessions.write().map_err(|_| {
            RedisError::String("Failed to acquire write lock".to_string())
        })?;
        
        // Update the last accessed time if session exists
        if let Some(session) = sessions_map.get_mut(&session_id_str) {
            session.last_accessed = Utc::now();
            Ok(RedisValue::SimpleString(format!("Session exists: {}", session_id_str)))
        } else {
            // Create a new session if session ID exists in hashmap but not in our store
            let session = Session {
                id: session_id_str.clone(),
                user_key: key,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                data: HashMap::new(),
            };
            
            sessions_map.insert(session_id_str.clone(), session);
            Ok(RedisValue::SimpleString(format!("Session recreated: {}", session_id_str)))
        }
    } else {
        Err(RedisError::String("Unexpected result type from custom.get".to_string()))
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
        // Also remove from custom hashmap
        match ctx.call("custom.del", &[&session.user_key]) {
            Ok(_) => Ok(RedisValue::Integer(1)),
            Err(err) => {
                // Re-add the session since we failed to remove from custom hashmap
                sessions_map.insert(session_id.clone(), session);
                Err(RedisError::String(format!("Failed to call custom.del: {}", err)))
            }
        }
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
