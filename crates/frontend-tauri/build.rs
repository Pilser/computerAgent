fn main() {
    #[cfg(feature = "tauri-app")]
    tauri_build::try_build().expect("tauri_build must succeed");
}
