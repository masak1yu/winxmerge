fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("translations/")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/main.slint", config).unwrap();
}
