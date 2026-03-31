//! WASM entry point for the web build (Cloudflare Pages).
//! Desktop builds use src/main.rs (binary target).

#[cfg(target_arch = "wasm32")]
mod diff;
#[cfg(target_arch = "wasm32")]
mod highlight;
#[cfg(target_arch = "wasm32")]
mod models;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
slint::include_modules!();

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// WASM entry point — called by wasm-bindgen's __wbindgen_start after init().
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        wasm::run();
    }
}
