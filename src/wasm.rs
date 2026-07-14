//! WebAssembly bindings.
//!
//! Build with `--features wasm` (and `--target wasm32-unknown-unknown`). Exposes
//! [`WasmFilter`] to JavaScript via `wasm-bindgen`. File/network I/O is not
//! available on WASM — use [`WasmFilter::load_words`] with in-memory text instead.

use crate::Filter;
use wasm_bindgen::prelude::*;

/// JavaScript-facing wrapper around [`Filter`].
#[wasm_bindgen]
pub struct WasmFilter {
    inner: Filter,
}

#[wasm_bindgen]
impl WasmFilter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: Filter::new() }
    }

    /// Add a single sensitive word (`filter.addWord('赌博')`).
    #[wasm_bindgen(js_name = addWord)]
    pub fn add_word(&mut self, word: &str) {
        self.inner.add_word(word);
    }

    /// Add many words from a JS array of strings (`filter.addWords(['赌博','色情'])`).
    #[wasm_bindgen(js_name = addWords)]
    pub fn add_words(&mut self, words: &js_sys::Array) {
        let words: Vec<String> = words.iter().filter_map(|v| v.as_string()).collect();
        let refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
        self.inner.add_words(&refs);
    }

    /// Find the first match → `{ found: boolean, word: string }`.
    #[wasm_bindgen(js_name = findIn)]
    pub fn find_in(&self, text: &str) -> JsValue {
        let (found, word) = self.inner.find_in(text);
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"found".into(), &JsValue::from_bool(found)).ok();
        js_sys::Reflect::set(&obj, &"word".into(), &JsValue::from_str(&word)).ok();
        obj.into()
    }

    /// Find all matches → JS array of word strings.
    #[wasm_bindgen(js_name = findAll)]
    pub fn find_all(&self, text: &str) -> js_sys::Array {
        let words = self.inner.find_all(text);
        let arr = js_sys::Array::new();
        for word in words {
            arr.push(&JsValue::from_str(&word));
        }
        arr
    }

    /// Replace each matched character with `replacement`.
    pub fn replace(&self, text: &str, replacement: char) -> String {
        self.inner.replace(text, replacement)
    }

    /// Remove all matches completely.
    pub fn filter(&self, text: &str) -> String {
        self.inner.filter(text)
    }

    /// Load a dictionary from in-memory text (one word per line). This replaces
    /// the filesystem-based loaders, which are unavailable on WASM.
    #[wasm_bindgen(js_name = loadWords)]
    pub fn load_words(&mut self, content: &str) {
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.inner.add_word(trimmed);
            }
        }
    }
}

impl Default for WasmFilter {
    fn default() -> Self {
        Self::new()
    }
}
