fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target == "wasm32" {
        // WASM build: compile the minimal wasm-app UI
        slint_build::compile("ui/wasm-app.slint").unwrap();
    } else {
        // Desktop build: compile the full main UI with translations
        println!("cargo:rerun-if-changed=translations/");
        let config = slint_build::CompilerConfiguration::new()
            .with_bundled_translations("translations/")
            .with_default_translation_context(slint_build::DefaultTranslationContext::None);
        slint_build::compile_with_config("ui/main.slint", config).unwrap();

        // Embed app icon into the Windows executable (requires --features embed-icon)
        #[cfg(all(target_os = "windows", feature = "embed-icon"))]
        {
            let mut res = winres::WindowsResource::new();
            res.set_icon("assets/icons/app-icon.ico");
            res.compile().unwrap();
        }
    }
}
