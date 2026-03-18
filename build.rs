fn main() {
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
