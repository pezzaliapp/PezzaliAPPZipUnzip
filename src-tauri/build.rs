fn main() {
    // La cartella del motore non è versionata (la popola `npm run fetch-7zip`):
    // vuota, l'app usa il 7-Zip di sistema; assente, la build di Tauri fallirebbe.
    std::fs::create_dir_all("engine").expect("cartella engine");

    // Windows (MSVC): il manifest che carica Common Controls v6 va incorporato in
    // tutti gli eseguibili, test compresi; tauri-build lo aggiunge solo all'app e
    // i test terminerebbero con STATUS_ENTRYPOINT_NOT_FOUND.
    let mut windows = tauri_build::WindowsAttributes::new();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        windows = tauri_build::WindowsAttributes::new_without_app_manifest();
        let manifest = std::env::current_dir()
            .expect("cartella corrente")
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri-build")
}
