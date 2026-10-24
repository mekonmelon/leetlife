fn main() {
    // Only the desktop shell needs Tauri's codegen (context, capabilities, icons).
    #[cfg(feature = "desktop")]
    tauri_build::build();
}
