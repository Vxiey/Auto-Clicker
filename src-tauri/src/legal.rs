#[tauri::command]
pub fn terms_of_service_text() -> &'static str {
    include_str!("../../TERMS.md")
}

#[tauri::command]
pub fn disclaimer_text() -> &'static str {
    include_str!("../../DISCLAIMER.md")
}
