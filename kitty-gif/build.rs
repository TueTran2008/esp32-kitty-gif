fn main() {
    embuild::espidf::sysenv::output();
    // slint_build::compile("ui/app-window.slint").expect("Slint build failed");
    slint_build::compile_with_config(
        "ui/app-window.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();
}
