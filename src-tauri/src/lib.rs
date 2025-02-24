mod gdrive;
mod events;

use tauri::{AppHandle, Emitter, Manager};
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::string::ToString;
use std::time::Duration;
use tauri::utils::mime_type::MimeType;
use tokio::time::sleep;
use zip::ZipArchive;
use crate::events::{UnpackProgress, UpdateProgress, UpdateStatus};

const FOLDER_ID: &str = "1JUOctbsugh2IIEUCWcBkupXYVYoJMg4G";
const BASE_DIR: &str = "D:\\RfaD SE\\MO2";
const LOCAL_VERSION_FILE_NAME: &str = "version.txt";
const REMOTE_VERSION_FILE_NAME: &str = "remote_version.txt";
const LOCAL_UPDATE_FILE_NAME: &str = "update.zip";

async fn unpack(mut archive: ZipArchive<File>, output: String, app: &AppHandle) {
    sleep(Duration::from_millis(400)).await;

    let total_files = archive.len();
    for i in 0..total_files {
        let mut file = archive.by_index(i).unwrap();
        let outpath = PathBuf::from(output.clone()).join(file.enclosed_name().unwrap());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }

        let percentage = ((i + 1) as f64 / total_files as f64) * 100.0;
        app.emit("unpack:progress", UnpackProgress {
            percentage
        }).ok();
    }
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

    let patch_dir = format!("{}/mods/RFAD_PATCH", BASE_DIR);
    if std::fs::exists(&patch_dir).unwrap() {
        std::fs::remove_dir_all(&patch_dir).unwrap();
    } else {
        std::fs::create_dir_all(&patch_dir).unwrap();
    }

    let zip_file = File::open(zip_path).unwrap();
    let archive = ZipArchive::new(zip_file).unwrap();

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::UnpackStarted as u8
    }).ok();

    unpack(archive, patch_dir, &app).await;

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::UnpackFinished as u8
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

