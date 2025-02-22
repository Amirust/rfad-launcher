mod gdrive;
mod events;

use tauri::{AppHandle, Emitter, Manager};
use std::env;
use std::string::ToString;
use tauri::utils::mime_type::MimeType;
use crate::events::{UpdateProgress, UpdateStatus};

const FOLDER_ID: &str = "1JUOctbsugh2IIEUCWcBkupXYVYoJMg4G";
const BASE_DIR: &str = "D:\\RfaD SE\\MO2";
const LOCAL_VERSION_FILE_NAME: &str = "version.txt";
const REMOTE_VERSION_FILE_NAME: &str = "remote_version.txt";
const LOCAL_UPDATE_FILE_NAME: &str = "update.zip";

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
    let version_file_path = format!("{}/mods/RFAD_PATCH/{}", BASE_DIR, LOCAL_VERSION_FILE_NAME);
    if !std::path::Path::new(&version_file_path).exists() {
        return "NO_PATCH".to_string();
    }

    std::fs::read_to_string(version_file_path).unwrap()
}

#[tauri::command]
async fn get_remote_version(app: AppHandle) -> String {
    let drive = gdrive::GoogleDriveClient::new().await;
    let res = drive.list_files(FOLDER_ID).await;

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::DownloadStarted as u8
    }).ok();

    let remote_version_file_path = format!("{}/{}", BASE_DIR, REMOTE_VERSION_FILE_NAME);
    if let Some((id, _, _)) = res.iter().find(|(_, name, _)| name == "version") {
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

#[tauri::command]
async fn update(app: AppHandle) -> bool {
    let drive = gdrive::GoogleDriveClient::new().await;
    let res = drive.list_files(FOLDER_ID).await;

    // Get file with mime application/x-zip-compressed
    let (id, _, _) = res.iter().find(|(_, name, mime)| mime == "application/x-zip-compressed").unwrap();

    let zip_path = format!("{}/{}", BASE_DIR, LOCAL_UPDATE_FILE_NAME);

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::DownloadStarted as u8
    }).ok();

    drive.download_file(
        id,
        MimeType::OctetStream,
        zip_path.as_str(),
        app.clone()
    ).await.ok();

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::DownloadFinished as u8
    }).ok();

    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![download, get_local_version, get_remote_version, update])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

