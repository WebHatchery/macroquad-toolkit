//! WASM localStorage wrapper using sapp-jsutils
//! Only compiled for wasm32 target

use sapp_jsutils::JsObject;

extern "C" {
    fn storage_set_extern(key: JsObject, value: JsObject);
    fn storage_get_extern(key: JsObject) -> JsObject;
    fn storage_remove_extern(key: JsObject);
    fn storage_exists_extern(key: JsObject) -> bool;
}

/// Version handshake for the named miniquad storage browser plugin.
#[cfg(target_family = "wasm")]
#[no_mangle]
pub extern "C" fn storage_crate_version() -> u32 {
    1
}

pub fn storage_set(key: &str, value: &str) {
    let js_key = JsObject::string(key);
    let js_value = JsObject::string(value);
    unsafe { storage_set_extern(js_key, js_value) };
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
