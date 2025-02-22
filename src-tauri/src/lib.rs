mod gdrive;

use tauri::{AppHandle, Emitter, Manager};
use std::env;
use std::string::ToString;
use tauri::utils::mime_type::MimeType;

const FOLDER_ID: &str = "1JUOctbsugh2IIEUCWcBkupXYVYoJMg4G";
const BASE_DIR: &str = "D:\\RfaD SE\\MO2";

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

#[tauri::command]
fn get_local_version() -> String {
    let version_file_path = format!("{}/mods/RFAD_PATCH/version.txt", BASE_DIR);
    if !std::path::Path::new(&version_file_path).exists() {
        return "NO_PATCH".to_string();
    }

    std::fs::read_to_string(version_file_path).unwrap()
}

#[tauri::command]
async fn get_remote_version(app: AppHandle) -> String {
    let drive = gdrive::GoogleDriveClient::new().await;
    let res = drive.list_files(FOLDER_ID).await;

    let remote_version_file_path = format!("{}/remote_version.txt", BASE_DIR);
    if let Some((id, _)) = res.iter().find(|(_, name)| name == "version") {
        drive.download_file(
            id,
            MimeType::Txt,
            remote_version_file_path.as_str(),
            app
        ).await.expect("Error downloading version file");

        let version = std::fs::read_to_string(&remote_version_file_path).unwrap();

        std::fs::remove_file(remote_version_file_path).unwrap();
        version
    } else {
        "NO_PATCH".to_string()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, download, get_local_version, get_remote_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

