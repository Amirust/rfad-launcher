mod gdrive;
mod events;

use tauri::{AppHandle, Emitter, Manager};
use std::{
    env,
    fs,
    fs::File,
    fs::OpenOptions,
    io::{Error, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    time::Duration,
};
use tauri::utils::mime_type::MimeType;
use tokio::time::sleep;
use zip::ZipArchive;
use crate::events::{UnpackProgress, UpdateProgress, UpdateStatus};

const FOLDER_ID: &str = "1JUOctbsugh2IIEUCWcBkupXYVYoJMg4G";
const LOCAL_VERSION_FILE_NAME: &str = "version.txt";
const REMOTE_VERSION_FILE_NAME: &str = "remote_version.txt";
const LOCAL_UPDATE_FILE_NAME: &str = "update.zip";
const MODS_JSON_FILE_NAME: &str = "launcher-mods.json";

fn exe_dir() -> PathBuf {
    env::current_exe()
        .expect("failed to get current exe path")
        .parent()
        .expect("failed to get exe parent directory")
        .to_path_buf()

    // return PathBuf::from("D:\\RfaD SE");
}

fn base_dir() -> PathBuf {
    exe_dir().join("MO2")
}

fn profile_dir() -> PathBuf {
    base_dir().join("profiles").join("RfaD SE 5.2")
}

#[tauri::command]
fn is_path_exist() -> bool {
    let path = exe_dir().join("MO2");
    path.exists()
}

async fn unpack(mut archive: ZipArchive<File>, output: PathBuf, app: &AppHandle) {
    sleep(Duration::from_millis(400)).await;
    let total_files = archive.len();

    for i in 0..total_files {
        let mut file = archive.by_index(i).unwrap();
        let outpath = output.join(file.enclosed_name().unwrap());

        if file.is_dir() {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut outfile = File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }

        let percentage = ((i + 1) as f64 / total_files as f64) * 100.0;
        app.emit("unpack:progress", UnpackProgress { percentage }).ok();
    }
}

async fn new_load_order() -> Result<String, ()> {
    let drive = gdrive::GoogleDriveClient::new().await;
    let files = drive.list_files(FOLDER_ID).await;
    let (id, _, _) = files.iter()
        .find(|(_, name, _)| name == "modlist")
        .ok_or(())?;
    let txt = drive.load_text(id).await.map_err(|_| ())?;
    if txt.is_empty() { Err(()) } else { Ok(txt) }
}

fn remove_whitespace(s: &str) -> String {
    s.trim().replace('\u{FEFF}', "")
}

fn update_modlist() {
    let path = profile_dir().join("modlist.txt");
    let mut file = OpenOptions::new().read(true).write(true).open(&path)
        .expect("cannot open modlist.txt");

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    let new_content = format!("+RFAD_PATCH\n{}", content.replace("+RFAD_PATCH\n", ""));
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(new_content.as_bytes()).unwrap();
    file.set_len(new_content.len() as u64).unwrap();
}

async fn update_order(path: &PathBuf, new_list: &str, separator: &str) -> Result<(), Error> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mod_list: Vec<String> = if separator == "Requiem for the Indifferent.esp" {
        new_list.lines().map(|l| l.to_string()).collect()
    } else {
        new_list.lines().map(|l| format!("*{}", l)).collect()
    };

    for entry in &mod_list {
        content = content.replace(entry, "");
    }

    if let Some((head, tail)) = content.split_once(separator) {
        let joined = mod_list.join("\n");
        let updated = format!("{}\n{}\n{}{}", head.trim_end(), joined, separator, tail);
        file.seek(SeekFrom::Start(0))?;
        file.write_all(updated.as_bytes())?;
        file.set_len(updated.len() as u64)?;
    }

    Ok(())
}

#[tauri::command]
async fn download(app: AppHandle, id: &str, file_name: &str) -> Result<String, ()> {
    let drive = gdrive::GoogleDriveClient::new().await;
    let mime = if file_name.ends_with(".txt") {
        MimeType::Txt
    } else {
        MimeType::OctetStream
    };
    let out_path = exe_dir().join(file_name);
    let res = drive.download_file(id, mime, out_path.to_str().unwrap(), app).await;
    Ok(format!("Downloaded: {:?}", res))
}

#[tauri::command]
fn get_local_version() -> String {
    let path = base_dir()
        .join("mods")
        .join("RFAD_PATCH")
        .join(LOCAL_VERSION_FILE_NAME);
    if !path.exists() {
        return "NO_PATCH".into();
    }
    fs::read_to_string(path).unwrap_or_else(|_| "NO_PATCH".into())
}

#[tauri::command]
async fn get_remote_version(app: AppHandle) -> String {
    let drive = gdrive::GoogleDriveClient::new().await;
    let files = drive.list_files(FOLDER_ID).await;

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::DownloadStarted as u8
    }).ok();

    if let Some((id, _, _)) = files.iter()
        .find(|(_, name, _)| name == "version")
    {
        let tmp = base_dir().join(REMOTE_VERSION_FILE_NAME);
        drive.download_file(id, MimeType::Txt, tmp.to_str().unwrap(), app.clone())
            .await
            .ok();
        let ver = fs::read_to_string(&tmp).unwrap_or_else(|_| "NO_PATCH".into());
        let _ = fs::remove_file(&tmp);
        ver
    } else {
        "NO_PATCH".into()
    }
}

#[tauri::command]
async fn update(app: AppHandle) -> bool {
    let drive = gdrive::GoogleDriveClient::new().await;
    let files = drive.list_files(FOLDER_ID).await;
    let (zip_id, _, _) = files.iter()
        .find(|(_, _, mime)| *mime == "application/x-zip-compressed")
        .expect("zip not found");

    let zip_path = base_dir().join(LOCAL_UPDATE_FILE_NAME);
    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::DownloadStarted as u8
    }).ok();
    let _ = drive.download_file(
        zip_id,
        MimeType::OctetStream,
        zip_path.to_str().unwrap(),
        app.clone(),
    ).await;

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::DownloadFinished as u8
    }).ok();

    let patch_dir = base_dir().join("mods").join("RFAD_PATCH");
    if patch_dir.exists() {
        fs::remove_dir_all(&patch_dir).unwrap();
    }
    fs::create_dir_all(&patch_dir).unwrap();

    let zip_file = File::open(&zip_path).unwrap();
    let archive = ZipArchive::new(zip_file).unwrap();
    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::UnpackStarted as u8
    }).ok();
    unpack(archive, patch_dir.clone(), &app).await;
    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::UnpackFinished as u8
    }).ok();

    let new_list = remove_whitespace(&new_load_order().await.unwrap_or_default());
    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::LoadOrderUpdateStarted as u8
    }).ok();

    update_modlist();
    if !new_list.is_empty() {
        let plugins_txt = profile_dir().join("plugins.txt");
        let loadorder_txt = profile_dir().join("loadorder.txt");
        update_order(&plugins_txt, &new_list, "Requiem for the Indifferent.esp")
            .await
            .expect("Error updating plugins.txt");
        update_order(&loadorder_txt, &new_list, "Requiem for the Indifferent.esp")
            .await
            .expect("Error updating loadorder.txt");
    }

    app.emit("update:progress", UpdateProgress {
        status: UpdateStatus::LoadOrderUpdateFinished as u8
    }).ok();

    let _ = fs::remove_file(&zip_path);
    true
}

#[tauri::command]
fn start_game() {
    let exe = base_dir().join("ModOrganizer.exe");
    std::process::Command::new(exe)
        .current_dir(&base_dir())
        .arg("moshortcut://:SKSE")
        .spawn()
        .expect("Failed to start game");
}

#[tauri::command]
fn open_explorer() {
    let base = base_dir();
    let parent = base.parent().expect("no parent dir");

    std::process::Command::new("explorer")
        .arg(parent)
        .spawn()
        .expect("Failed to open explorer");
}

#[tauri::command]
fn open_mo2() {
    let exe = base_dir().join("ModOrganizer.exe");
    std::process::Command::new(exe)
        .current_dir(&base_dir())
        .spawn()
        .expect("Failed to start MO2");
}

#[tauri::command]
async fn load_json_mods(app: AppHandle) -> String {
    let drive = gdrive::GoogleDriveClient::new().await;
    let files = drive.list_files(FOLDER_ID).await;

    if let Some((id, _, _)) = files.iter().find(|(_, name, _)| name == MODS_JSON_FILE_NAME) {
        let tmp = base_dir().join(MODS_JSON_FILE_NAME);
        drive.download_file(id, MimeType::Json, tmp.to_str().unwrap(), app.clone())
            .await
            .ok();
        let json = fs::read_to_string(&tmp).unwrap_or_else(|_| "[]".into());
        let _ = fs::remove_file(&tmp);
        json
    } else {
        "[]".into()
    }

}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            download, get_local_version, get_remote_version,
            update, start_game, open_explorer,
            open_mo2, is_path_exist, load_json_mods
    ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}