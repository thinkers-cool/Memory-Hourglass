#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    memhg_lib::configure_builder(tauri::Builder::default())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
