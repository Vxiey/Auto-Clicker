fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("PROFILE").as_deref() == Ok("release")
    {
        // Match Tauri's recommended Windows GUI subsystem for release builds so
        // launching VxClick does not allocate a console window.
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
    }

    tauri_build::build()
}
