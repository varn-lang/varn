use std::sync::atomic::{AtomicBool, Ordering};

pub struct Settings {
    inlay_hints: AtomicBool,
    code_lens: AtomicBool,
    references_lens: AtomicBool,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            inlay_hints: AtomicBool::new(true),
            code_lens: AtomicBool::new(true),
            references_lens: AtomicBool::new(true),
        }
    }

    pub fn inlay_hints_enabled(&self) -> bool {
        self.inlay_hints.load(Ordering::Relaxed)
    }

    pub fn code_lens_enabled(&self) -> bool {
        self.code_lens.load(Ordering::Relaxed)
    }

    pub fn references_lens_enabled(&self) -> bool {
        self.references_lens.load(Ordering::Relaxed)
    }

    pub fn apply(&self, value: &serde_json::Value) {
        if let Some(enabled) = Self::inlay_hints_enabled_in(value) {
            self.inlay_hints.store(enabled, Ordering::Relaxed);
        }
        if let Some(enabled) = Self::code_lens_enabled_in(value) {
            self.code_lens.store(enabled, Ordering::Relaxed);
        }
        if let Some(enabled) = Self::references_lens_enabled_in(value) {
            self.references_lens.store(enabled, Ordering::Relaxed);
        }
    }

    pub fn inlay_hints_enabled_in(value: &serde_json::Value) -> Option<bool> {
        value
            .pointer("/vn/inlayHints/enabled")
            .or_else(|| value.pointer("/inlayHints/enabled"))
            .and_then(serde_json::Value::as_bool)
    }

    pub fn code_lens_enabled_in(value: &serde_json::Value) -> Option<bool> {
        value
            .pointer("/vn/codeLens/enabled")
            .or_else(|| value.pointer("/codeLens/enabled"))
            .and_then(serde_json::Value::as_bool)
    }

    pub fn references_lens_enabled_in(value: &serde_json::Value) -> Option<bool> {
        value
            .pointer("/vn/codeLens/referencesEnabled")
            .or_else(|| value.pointer("/codeLens/referencesEnabled"))
            .or_else(|| value.pointer("/vn/references/codeLensEnabled"))
            .and_then(serde_json::Value::as_bool)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}
