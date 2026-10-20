//! wasm-only bridge: publish game state to `window.__FTD_STATE__` each frame
//! for the Playwright harness and debugging (PLAN.md testing strategy).

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct TouchSnapshot {
    pub enabled: bool,
    pub steer: f32,
    pub brake: f32,
    pub low_seq: u32,
    pub high_seq: u32,
    pub nitro_seq: u32,
    pub jump_seq: u32,
    pub pause_seq: u32,
    pub difficulty_seq: u32,
    pub difficulty_index: i8,
    pub confirm_seq: u32,
    pub crt_seq: u32,
    pub large_seq: u32,
    pub restart_seq: u32,
    pub initials_seq: u32,
    pub initials: String,
    pub submit_seq: u32,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(default)]
pub struct LocalCalendar {
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub southern: bool,
}

impl Default for LocalCalendar {
    fn default() -> Self {
        Self {
            month: 3,
            day: 20,
            hour: 12,
            minute: 0,
            southern: false,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn local_calendar() -> LocalCalendar {
    use wasm_bindgen::JsCast;
    let global = js_sys::global();
    let Ok(function) = js_sys::Reflect::get(&global, &"__ftdLocalCalendar".into()) else {
        return LocalCalendar::default();
    };
    let Ok(function) = function.dyn_into::<js_sys::Function>() else {
        return LocalCalendar::default();
    };
    function
        .call0(&global)
        .ok()
        .and_then(|value| value.as_string())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn local_calendar() -> LocalCalendar {
    LocalCalendar::default()
}

#[cfg(target_arch = "wasm32")]
pub fn touch_snapshot() -> TouchSnapshot {
    use wasm_bindgen::JsCast;
    let global = js_sys::global();
    let Ok(function) = js_sys::Reflect::get(&global, &"__ftdReadTouch".into()) else {
        return TouchSnapshot::default();
    };
    let Ok(function) = function.dyn_into::<js_sys::Function>() else {
        return TouchSnapshot::default();
    };
    function
        .call0(&global)
        .ok()
        .and_then(|value| value.as_string())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn touch_snapshot() -> TouchSnapshot {
    TouchSnapshot::default()
}

#[cfg(target_arch = "wasm32")]
pub fn coarse_pointer() -> bool {
    use wasm_bindgen::JsCast;
    let global = js_sys::global();
    let Ok(function) = js_sys::Reflect::get(&global, &"__ftdCoarsePointer".into()) else {
        return false;
    };
    let Ok(function) = function.dyn_into::<js_sys::Function>() else {
        return false;
    };
    function
        .call0(&global)
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn coarse_pointer() -> bool {
    false
}

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
pub fn audio_running() -> bool {
    use wasm_bindgen::JsCast;
    let global = js_sys::global();
    let Ok(function) = js_sys::Reflect::get(&global, &"__ftdAudioRunning".into()) else {
        return false;
    };
    let Ok(function) = function.dyn_into::<js_sys::Function>() else {
        return false;
    };
    function
        .call0(&global)
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn audio_running() -> bool {
    true
}

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
        assert!(html.contains("__ftdAudioRunning"));
        assert!(html.contains("'pointerdown', 'touchstart', 'keydown'"));
    }

    #[test]
    fn touch_bridge_is_loaded_before_wasm_and_uses_sequence_counters() {
        let html = include_str!("../index.html");
        let touch = html.find("__ftdReadTouch").expect("touch bridge");
        let wasm = html
            .find("data-trunk rel=\"rust\"")
            .expect("trunk wasm link");
        assert!(touch < wasm);
        assert!(html.contains("jump_seq"));
        assert!(html.contains("difficulty_seq"));
        assert!(html.contains("setPointerCapture"));
        assert!(html.contains("'pointercancel', 'lostpointercapture'"));
        assert!(html.contains("env(safe-area-inset-left)"));
        assert!(html.contains(">COLD-AIR</button>"));
        assert!(html.contains("__ftdCoarsePointer"));
        assert!(html.contains("id=\"touch-pause\""));
        assert!(html.contains("id=\"touch-restart\""));
        assert!(html.contains("pulse('pause_seq')"));
        assert!(html.contains("pulse('restart_seq')"));
        assert!(html.contains("body.touch-enabled canvas"));
        assert!(html.contains("id=\"touch-large\""));
        assert!(html.contains("id=\"touch-large-driving\""));
        assert!(html.contains("large_seq"));
        assert!(html.contains("body.large-screen:not(.direct-game) canvas"));
        assert!(html.contains("transition: transform"));
    }

    #[test]
    fn touch_controls_suppress_ios_callouts_without_blocking_text_inputs() {
        let html = include_str!("../index.html");
        let callout = html
            .find("-webkit-touch-callout: none")
            .expect("iOS callout suppression");
        let selector_start = html[..callout].rfind('}').map_or(0, |index| index + 1);
        let selector_end = html[..callout].rfind('{').expect("callout selector");
        let callout_selector = &html[selector_start..selector_end];

        assert!(callout_selector.contains(".touch-button"));
        assert!(callout_selector.contains("#touch-screen-tap"));
        assert!(!callout_selector.contains("#touch-initials"));
        assert!(html.contains("-webkit-user-select: none"));
        assert!(html.contains("event.target.closest(touchControlSelector)"));
        assert!(html.contains("addEventListener('contextmenu'"));
    }

    #[test]
    fn local_calendar_bridge_precedes_wasm_and_detects_hemisphere() {
        let html = include_str!("../index.html");
        let calendar = html.find("__ftdLocalCalendar").expect("calendar bridge");
        let wasm = html
            .find("data-trunk rel=\"rust\"")
            .expect("trunk wasm link");
        assert!(calendar < wasm);
        assert!(html.contains("America\\/(Argentina\\/|Sao_Paulo"));
        assert!(html.contains("now.getMonth() + 1"));
        assert!(html.contains("now.getHours()"));
        assert!(html.contains("now.getMinutes()"));
    }

    #[test]
    fn touch_bridge_survives_unavailable_local_storage() {
        let html = include_str!("../index.html");
        assert!(html.contains("try { return localStorage.getItem(storageKey); } catch (_)"));
        assert!(html.contains("try { localStorage.setItem(storageKey"));
        assert!(html.contains("applyEnabled();"));
    }
}
