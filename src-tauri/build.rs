fn main() {
    // La cartella del motore non è versionata (la popola `npm run fetch-7zip`):
    // vuota, l'app usa il 7-Zip di sistema; assente, la build di Tauri fallirebbe.
    std::fs::create_dir_all("engine").expect("cartella engine");
    tauri_build::build()
}
