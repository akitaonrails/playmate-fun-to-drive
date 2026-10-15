//! wasm-only bridge: publish game state to `window.__FTD_STATE__` each frame
//! for the Playwright harness and debugging (PLAN.md testing strategy).

#![cfg(target_arch = "wasm32")]

pub fn publish_state(json: &str) {
    use wasm_bindgen::JsCast;
    let global = js_sys::global();
    if let Ok(f) = js_sys::Reflect::get(&global, &js_sys::JsString::from("__ftdPublishState")) {
        if let Ok(f) = f.dyn_into::<js_sys::Function>() {
            let _ = f.call1(&global, &js_sys::JsString::from(json));
        }
    }
}
