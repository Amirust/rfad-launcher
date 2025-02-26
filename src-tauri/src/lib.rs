mod gdrive;
mod events;

use tauri::{AppHandle, Emitter, Manager};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Error, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::string::ToString;
use std::time::Duration;
use tauri::utils::mime_type::MimeType;
use tokio::time::sleep;
use zip::ZipArchive;
use crate::events::{UnpackProgress, UpdateProgress, UpdateStatus};

const FOLDER_ID: &str = "1JUOctbsugh2IIEUCWcBkupXYVYoJMg4G";
const BASE_DIR: &str = "D:/RfaD SE/MO2";
const PROFILE_DIR: &str = "D:\\RfaD SE\\MO2\\profiles\\RfaD SE 5.2";
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

async fn new_load_order() -> Result<String, ()> {
    let drive = gdrive::GoogleDriveClient::new().await;
    let res = drive.list_files(FOLDER_ID).await;

    let (id, _, _) = res.iter().find(|(_, name, mime)| name == "modlist").unwrap();

    let new_order = drive.load_text(
        id,
    ).await.expect("Error downloading load order file");

    if new_order.len() == 0 {
        Err(())
    } else {
        Ok(new_order)
    }
}

fn remove_whitespace(s: &str) -> String {
    s.trim().replace("\u{FEFF}", "")
}

fn update_modlist() {
    let path_to_file = format!("{}/modlist.txt", PROFILE_DIR);
    let mut file = OpenOptions::new().read(true).write(true).open(path_to_file).unwrap();

    let mut loadorder = String::new();
    file.read_to_string(&mut loadorder).unwrap();

    let new_modlist = format!("+RFAD_PATCH\n{}", loadorder.replace("+RFAD_PATCH\n", ""));

    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(new_modlist.as_bytes()).unwrap();
    file.set_len(new_modlist.len() as u64).unwrap();
}

async fn update_order(path_to_file: &str, new_list: &str, separator: &str) -> Result<(), Error> {
    let mut file = OpenOptions::new().read(true).write(true).open(path_to_file)?;

    let mut loadorder = String::new();
    file.read_to_string(&mut loadorder)?;

    let mod_list: Vec<String> = if separator == "Requiem for the Indifferent.esp" {
        new_list.lines().map(|x| x.to_string()).collect()
    } else {
        new_list.lines().map(|x| format!("*{}", x)).collect()
    };

    for x in &mod_list {
        loadorder = loadorder.replace(x, "");
    }

    if let Some((head, tail)) = loadorder.split_once(separator) {
        let new_list_str = mod_list.join("\n");
        let updated_content = format!("{}\n{}\n{}{}", head.trim_end(), new_list_str, separator, tail);

        file.seek(SeekFrom::Start(0))?;
        file.write_all(updated_content.as_bytes())?;
        file.set_len(updated_content.len() as u64)?;
    }

    Ok(())
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
       match drive.download_file(
           id,
           MimeType::Txt,
           remote_version_file_path.as_str(),
           app
       ).await {
          Ok(_) => {},
          Err(_) => return "NO_DIR".to_string()
       }

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

    let new_list = remove_whitespace(&*new_load_order().await.unwrap());

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::LoadOrderUpdateStarted as u8
    }).ok();

    update_modlist();

    if new_list.len() > 0 {
        update_order(
            format!("{}/plugins.txt", PROFILE_DIR).as_str(),
            new_list.as_str(),
            "Requiem for the Indifferent.esp"
        ).await.expect("Error updating load order");

        update_order(
            format!("{}/loadorder.txt", PROFILE_DIR).as_str(),
            new_list.as_str(),
            "Requiem for the Indifferent.esp"
        ).await.expect("Error updating load order");
    }

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::LoadOrderUpdateFinished as u8
    }).ok();

    true
}

#[tauri::command]
fn start_game() {
    let _ = std::process::Command::new("D:\\RfaD SE\\MO2\\ModOrganizer.exe")
        .current_dir("D:\\RfaD SE\\MO2")
        .arg("moshortcut://:SKSE")
        .spawn()
        .expect("Failed to start game");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![download, get_local_version, get_remote_version, update, start_game])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

