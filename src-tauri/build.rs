fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");

    tauri_build::build();
}
