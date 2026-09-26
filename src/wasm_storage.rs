//! WASM localStorage wrapper using sapp-jsutils
//! Only compiled for wasm32 target

use sapp_jsutils::JsObject;

extern "C" {
    fn storage_set_extern(key: JsObject, value: JsObject) -> bool;
    fn storage_get_extern(key: JsObject) -> JsObject;
    fn storage_remove_extern(key: JsObject);
    fn storage_exists_extern(key: JsObject) -> bool;
    fn storage_read_checked_extern(key: JsObject) -> JsObject;
    fn storage_remove_checked_extern(key: JsObject) -> JsObject;
    fn storage_lock_request_extern(name: JsObject) -> u32;
    fn storage_lock_poll_extern(id: u32) -> JsObject;
    fn storage_lock_release_extern(id: u32);
}

#[derive(serde::Deserialize)]
struct CheckedStorage {
    value: Option<String>,
    error: Option<String>,
}

fn response<T: serde::de::DeserializeOwned>(value: JsObject) -> Result<T, String> {
    if value.is_nil() {
        return Err("Browser storage returned no checked response".into());
    }
    let mut json = String::new();
    value.to_string(&mut json);
    serde_json::from_str(&json).map_err(|error| format!("Browser storage response: {error}"))
}

/// Distinguishes an absent key from blocked or otherwise unreadable storage.
pub fn storage_read_checked(key: &str) -> Result<Option<String>, String> {
    let result: CheckedStorage =
        response(unsafe { storage_read_checked_extern(JsObject::string(key)) })?;
    match result.error {
        Some(error) => Err(format!("Browser storage read: {error}")),
        None => Ok(result.value),
    }
}

/// Idempotent deletion with an observable failure result.
pub fn storage_remove_checked(key: &str) -> Result<(), String> {
    let result: CheckedStorage =
        response(unsafe { storage_remove_checked_extern(JsObject::string(key)) })?;
    match result.error {
        Some(error) => Err(format!("Browser storage removal: {error}")),
        None => Ok(()),
    }
}

/// Starts an asynchronous exclusive lease. Poll it in later frames.
pub(crate) fn request_writer(name: &str) -> u32 {
    unsafe { storage_lock_request_extern(JsObject::string(name)) }
}

pub(crate) fn poll_writer(id: u32) -> Result<super::persistence::WriterStatus, String> {
    #[derive(serde::Deserialize)]
    struct Lease {
        status: String,
        error: Option<String>,
    }
    let lease: Lease = response(unsafe { storage_lock_poll_extern(id) })?;
    match lease.status.as_str() {
        "pending" => Ok(super::persistence::WriterStatus::Pending),
        "ready" => Ok(super::persistence::WriterStatus::Ready),
        "busy" => Ok(super::persistence::WriterStatus::Busy),
        _ => Err(lease
            .error
            .unwrap_or_else(|| "Save writer lease failed".into())),
    }
}

pub(crate) fn release_writer(id: u32) {
    unsafe { storage_lock_release_extern(id) };
}

/// Version handshake for the named miniquad storage browser plugin.
#[cfg(target_family = "wasm")]
#[no_mangle]
pub extern "C" fn storage_crate_version() -> u32 {
    1
}

pub fn storage_set(key: &str, value: &str) -> Result<(), String> {
    let js_key = JsObject::string(key);
    let js_value = JsObject::string(value);
    if unsafe { storage_set_extern(js_key, js_value) } {
        Ok(())
    } else {
        Err("Browser storage rejected the write (it may be full or blocked)".to_owned())
    }
}

pub fn storage_get(key: &str) -> Option<String> {
    // A missing key must return None, never panic. The JS `getItem` yields
    // `null` for an absent key, but that comes back as a non-nil JsObject whose
    // `to_string` calls `js_string_length(undefined)` and throws — which unwinds
    // through the frame and poisons miniquad's event-handler RefCell (surfacing
    // later as a bogus "already borrowed" panic on the next focus event). Gate
    // the read on the existence check, which returns a plain bool and can't trip
    // that path. (Hit on first-run web loads, before anything has been saved.)
    if !storage_exists(key) {
        return None;
    }
    let js_key = JsObject::string(key);
    let result = unsafe { storage_get_extern(js_key) };
    if result.is_nil() {
        None
    } else {
        let mut buf = String::new();
        result.to_string(&mut buf);
        Some(buf)
    }
}

pub fn storage_remove(key: &str) {
    let js_key = JsObject::string(key);
    unsafe { storage_remove_extern(js_key) };
}

pub fn storage_exists(key: &str) -> bool {
    let js_key = JsObject::string(key);
    unsafe { storage_exists_extern(js_key) }
}
