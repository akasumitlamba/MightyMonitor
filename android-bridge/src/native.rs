//! JNI exports. Handles index owned Rust clients; no pointer is stored in Java.
//! JNI entry names require no_mangle; all actual operations use safe jni APIs.
use crate::Client;
use jni::{
    objects::{JClass, JString},
    sys::{jbyteArray, jlong},
    JNIEnv,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, Ordering},
        Mutex, OnceLock,
    },
};
static CLIENTS: OnceLock<Mutex<HashMap<i64, Client>>> = OnceLock::new();
static NEXT_ID: AtomicI64 = AtomicI64::new(1);
fn clients() -> &'static Mutex<HashMap<i64, Client>> {
    CLIENTS.get_or_init(|| Mutex::new(HashMap::new()))
}
#[no_mangle]
pub extern "system" fn Java_com_mightymonitor_app_NativeBridge_connect(
    mut env: JNIEnv,
    _: JClass,
    address: JString,
) -> jlong {
    let result = (|| -> Result<i64, String> {
        let address: String = env.get_string(&address).map_err(|e| e.to_string())?.into();
        let mut clients = clients().lock().unwrap_or_else(|e| e.into_inner());
        if clients.len() >= 4 {
            return Err("Too many active connections".into());
        }
        let client = Client::connect(&address).map_err(|e| e.to_string())?;
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        if id <= 0 {
            return Err("Connection handles exhausted".into());
        }
        clients.insert(id, client);
        Ok(id)
    })();
    match result {
        Ok(id) => id,
        Err(error) => {
            let _ = env.throw_new("java/lang/IllegalArgumentException", error);
            0
        }
    }
}
#[no_mangle]
pub extern "system" fn Java_com_mightymonitor_app_NativeBridge_poll(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
) -> jbyteArray {
    let event = clients()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&id)
        .and_then(Client::poll);
    match event {
        Some(bytes) => match env.byte_array_from_slice(&bytes) {
            Ok(array) => array.into_raw(),
            Err(error) => {
                let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
                std::ptr::null_mut()
            }
        },
        None => std::ptr::null_mut(),
    }
}
#[no_mangle]
pub extern "system" fn Java_com_mightymonitor_app_NativeBridge_disconnect(
    _env: JNIEnv,
    _: JClass,
    id: jlong,
) {
    // Release the table lock before joining the worker; other clients can poll.
    let client = clients()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&id);
    drop(client);
}
