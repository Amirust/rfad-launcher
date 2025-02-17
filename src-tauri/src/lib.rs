mod gdrive;

use tauri::{AppHandle, Emitter, Manager};
use std::env;
use std::string::ToString;
use tauri::utils::mime_type::MimeType;

const FOLDER_ID: &str = "1JUOctbsugh2IIEUCWcBkupXYVYoJMg4G";

#[tauri::command]
async fn greet(name: &str) -> Result<String, ()> {
    let drive = gdrive::GoogleDriveClient::new().await;
    let res = drive.list_files(FOLDER_ID).await;

    Ok(format!("Ку ёпта, {}! Раст сасёт\nFiles: {:?}", name, res))
}

#[tauri::command]
async fn download(app: AppHandle, id: &str, file_name: &str) -> Result<String, ()> {
    let drive = gdrive::GoogleDriveClient::new().await;
    let is_txt = file_name.ends_with(".txt");

    let res = drive.download_file(
        id,
        if is_txt { MimeType::Txt } else { MimeType::OctetStream },
        format!("D:\\Projects\\rfad-launcher\\{}", file_name).as_str(),
        app
    ).await;

    Ok(format!("Downloaded: {:?}", res))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, download])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

