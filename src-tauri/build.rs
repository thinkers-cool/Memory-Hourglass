fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
    )
    .expect("failed to run tauri-build");

    embed_windows_manifest_for_tests();
}

#[cfg(windows)]
fn embed_windows_manifest_for_tests() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }

    let manifest = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"),
    )
    .join("windows-app-manifest.xml");

    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        manifest.display()
    );
}

#[cfg(not(windows))]
fn embed_windows_manifest_for_tests() {}
