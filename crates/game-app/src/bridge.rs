//! wasm-only bridge: publish game state to `window.__FTD_STATE__` each frame
//! for the Playwright harness and debugging (PLAN.md testing strategy).

#[cfg(target_arch = "wasm32")]
pub fn publish_state(json: &str) {
    use wasm_bindgen::JsCast;
    let global = js_sys::global();
    if let Ok(f) = js_sys::Reflect::get(&global, &js_sys::JsString::from("__ftdPublishState")) {
        if let Ok(f) = f.dyn_into::<js_sys::Function>() {
            let _ = f.call1(&global, &js_sys::JsString::from(json));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn state_publisher_available() -> bool {
    let global = js_sys::global();
    js_sys::Reflect::get(&global, &js_sys::JsString::from("__ftdPublishState"))
        .is_ok_and(|value| value.is_function())
}

#[cfg(target_arch = "wasm32")]
pub fn resume_audio() {
    use wasm_bindgen::JsCast;
    let global = js_sys::global();
    if let Ok(function) = js_sys::Reflect::get(&global, &"__ftdResumeAudio".into()) {
        if let Ok(function) = function.dyn_into::<js_sys::Function>() {
            let _ = function.call0(&global);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resume_audio() {}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
pub fn load_local(key: &str) -> (Option<String>, bool) {
    let Some(storage) = storage() else {
        return (None, false);
    };
    match storage.get_item(key) {
        Ok(value) => (value, true),
        Err(_) => (None, false),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_local(_key: &str) -> (Option<String>, bool) {
    (None, false)
}

#[cfg(target_arch = "wasm32")]
pub fn save_local(key: &str, value: &str) -> bool {
    storage().is_some_and(|storage| storage.set_item(key, value).is_ok())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_local(_key: &str, _value: &str) -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
pub fn query_param(name: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    web_sys::UrlSearchParams::new_with_str(&search)
        .ok()?
        .get(name)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn query_param(_name: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn audio_context_retry_shim_loads_before_wasm() {
        let html = include_str!("../index.html");
        let shim = html.find("TrackedAudioContext").expect("audio retry shim");
        let wasm = html
            .find("data-trunk rel=\"rust\"")
            .expect("trunk wasm link");
        assert!(
            shim < wasm,
            "the shim must wrap AudioContext before Bevy starts"
        );
        assert!(html.contains("__ftdResumeAudio"));
        assert!(html.contains("'pointerdown', 'touchstart', 'keydown'"));
    }
}
