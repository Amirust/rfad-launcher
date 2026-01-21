mod events;
mod gdrive;

use crate::events::{UnpackProgress, UpdateProgress, UpdateStatus};
use std::{
    env, fs,
    fs::File,
    fs::OpenOptions,
    io::{Error, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    time::{Duration, SystemTime},
};
use futures::StreamExt;
use tauri::utils::mime_type::MimeType;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use zip::ZipArchive;

const FOLDER_ID: &str = "1JUOctbsugh2IIEUCWcBkupXYVYoJMg4G";
const LOCAL_VERSION_FILE_NAME: &str = "version.txt";
const REMOTE_VERSION_FILE_NAME: &str = "remote_version.txt";
const LOCAL_UPDATE_FILE_NAME: &str = "update.zip";
const MODS_JSON_FILE_NAME: &str = "launcher-mods.json";
const PATCHES_JSON_FILE_NAME: &str = "launcher-patches.json";

#[tauri::command]
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
    base_dir().join("profiles").join("RFAD_SE")
}

fn write_log(entry: &str) {
    let log_path = exe_dir().join("log.txt");
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "[{}] {}", ts, entry);
    }
}

fn sse_display_tweaks_path() -> PathBuf {
    let path = base_dir()
        .join("mods")
        .join("SSE Display Tweaks")
        .join("SKSE")
        .join("Plugins")
        .join("SSEDisplayTweaks.ini");
    write_log(&format!(
        "Resolved SSEDisplayTweaks.ini path: {}",
        path.display()
    ));
    path
}

fn skyrim_ini_path() -> PathBuf {
    let path = profile_dir().join("Skyrim.ini");
    write_log(&format!("Resolved Skyrim.ini path: {}", path.display()));
    path
}

fn skyrim_custom_ini_path() -> PathBuf {
    let path = profile_dir().join("SkyrimCustom.ini");
    write_log(&format!(
        "Resolved SkyrimCustom.ini path: {}",
        path.display()
    ));
    path
}

#[tauri::command]
fn is_path_exist() -> bool {
    let path = exe_dir().join("MO2");
    path.exists()
}

#[tauri::command]
fn get_framerate_limit() -> Result<u32, String> {
    let path = sse_display_tweaks_path();
    write_log(&format!(
        "Reading framerate limit from {}",
        path.display()
    ));

    let content = fs::read_to_string(&path).map_err(|e| {
        let msg = format!("Failed to read SSEDisplayTweaks.ini: {}", e);
        write_log(&msg);
        msg
    })?;

    match parse_framerate_limit(&content) {
        Ok(limit) => {
            write_log(&format!("Parsed FramerateLimit={}", limit));
            Ok(limit)
        }
        Err(err) => {
            write_log(&format!(
                "Failed to parse FramerateLimit from {}: {}",
                path.display(),
                err
            ));
            Err(err)
        }
    }
}

#[tauri::command]
fn get_voice_locale() -> Result<String, String> {
    let path = skyrim_ini_path();
    write_log(&format!(
        "Reading voice locale from {}",
        path.display()
    ));

    let content = fs::read_to_string(&path).map_err(|e| {
        let msg = format!("Failed to read Skyrim.ini: {}", e);
        write_log(&msg);
        msg
    })?;

    let locale = detect_voice_locale(&content);
    write_log(&format!("Detected voice locale: {}", locale));
    Ok(locale)
}

#[tauri::command]
fn update_game_settings(framerate: u32, voice: String) -> Result<(), String> {
    if voice != "en" && voice != "ru" {
        return Err("Voice must be 'en' or 'ru'".into());
    }

    write_log(&format!(
        "Updating game settings: framerate={} voice={}",
        framerate, voice
    ));

    let tweaks_path = sse_display_tweaks_path();
    let tweaks_content = fs::read_to_string(&tweaks_path).map_err(|e| {
        let msg = format!("Failed to read SSEDisplayTweaks.ini: {}", e);
        write_log(&msg);
        msg
    })?;
    let updated_tweaks = replace_framerate_limit(&tweaks_content, framerate).map_err(|e| {
        write_log(&format!(
            "Failed to replace FramerateLimit in {}: {}",
            tweaks_path.display(),
            e
        ));
        e
    })?;
    fs::write(&tweaks_path, updated_tweaks).map_err(|e| {
        let msg = format!("Failed to write SSEDisplayTweaks.ini: {}", e);
        write_log(&msg);
        msg
    })?;
    write_log(&format!(
        "Saved framerate limit to {}",
        tweaks_path.display()
    ));

    let skyrim_path = skyrim_ini_path();
    let skyrim_content = fs::read_to_string(&skyrim_path).map_err(|e| {
        let msg = format!("Failed to read Skyrim.ini: {}", e);
        write_log(&msg);
        msg
    })?;
    let updated_skyrim = replace_voice_locale(&skyrim_content, &voice).map_err(|e| {
        write_log(&format!(
            "Failed to replace voice locale in {}: {}",
            skyrim_path.display(),
            e
        ));
        e
    })?;
    fs::write(&skyrim_path, updated_skyrim).map_err(|e| {
        let msg = format!("Failed to write Skyrim.ini: {}", e);
        write_log(&msg);
        msg
    })?;
    write_log(&format!(
        "Saved voice locale '{}' to {}",
        voice,
        skyrim_path.display()
    ));

    let skyrim_custom_path = skyrim_custom_ini_path();
    let skyrim_custom_content = fs::read_to_string(&skyrim_custom_path).map_err(|e| {
        let msg = format!("Failed to read SkyrimCustom.ini: {}", e);
        write_log(&msg);
        msg
    })?;
    let updated_skyrim_custom = replace_voice_locale(&skyrim_custom_content, &voice).map_err(|e| {
        write_log(&format!(
            "Failed to replace voice locale in {}: {}",
            skyrim_custom_path.display(),
            e
        ));
        e
    })?;
    fs::write(&skyrim_custom_path, updated_skyrim_custom).map_err(|e| {
        let msg = format!("Failed to write SkyrimCustom.ini: {}", e);
        write_log(&msg);
        msg
    })?;
    write_log(&format!(
        "Saved voice locale '{}' to {}",
        voice,
        skyrim_custom_path.display()
    ));

    Ok(())
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
        app.emit("unpack:progress", UnpackProgress { percentage })
            .ok();
    }
}

async fn new_load_order() -> Result<String, ()> {
    let drive = gdrive::GoogleDriveClient::new().await;
    let files = drive.list_files(FOLDER_ID).await;
    let (id, _, _) = files
        .iter()
        .find(|(_, name, _)| name == "modlist")
        .ok_or(())?;
    let txt = drive.load_text(id).await.map_err(|_| ())?;
    if txt.is_empty() {
        Err(())
    } else {
        Ok(txt)
    }
}

fn remove_whitespace(s: &str) -> String {
    s.trim().replace('\u{FEFF}', "")
}

fn update_modlist() {
    let path = profile_dir().join("modlist.txt");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
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

fn parse_framerate_limit(content: &str) -> Result<u32, String> {
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("FramerateLimit") {
            if let Some((_, value)) = trimmed.split_once('=') {
                return value
                    .trim()
                    .parse::<u32>()
                    .map_err(|e| format!("Cannot parse FramerateLimit: {}", e));
            }
        }
    }

    Err("FramerateLimit not found".into())
}

fn detect_voice_locale(content: &str) -> String {
    if let Some(line) = content
        .lines()
        .find(|line| line.trim_start().starts_with("sResourceArchiveList2"))
    {
        if line.contains("Voices_ru0") {
            return "ru".into();
        }
        if line.contains("Voices_en0") {
            return "en".into();
        }
    }

    // Default to English if not found to avoid blocking the UI.
    "en".into()
}

fn replace_framerate_limit(content: &str, new_limit: u32) -> Result<String, String> {
    let mut replaced = false;

    let updated: String = content
        .split_inclusive('\n')
        .map(|segment| {
            let line = segment.trim_end_matches('\n');
            let trimmed = line.trim_start();

            if trimmed.starts_with("FramerateLimit")
                && !trimmed.starts_with(";FramerateLimit")
                && trimmed.contains('=')
            {
                replaced = true;
                let prefix_len = line.len() - trimmed.len();
                let prefix = &line[..prefix_len];
                let newline = if segment.ends_with('\n') { "\n" } else { "" };
                format!("{}FramerateLimit = {}{}", prefix, new_limit, newline)
            } else {
                segment.to_string()
            }
        })
        .collect();

    if replaced {
        Ok(updated)
    } else {
        Err("FramerateLimit not found in config".into())
    }
}

fn replace_voice_locale(content: &str, voice: &str) -> Result<String, String> {
    let mut replaced = false;
    let target_token = if voice == "ru" { "Voices_ru0" } else { "Voices_en0" };

    let updated: String = content
        .split_inclusive('\n')
        .map(|segment| {
            let line = segment.trim_end_matches('\n');
            let trimmed = line.trim_start();

            if trimmed.starts_with("sResourceArchiveList2") {
                replaced = true;
                let newline = if segment.ends_with('\n') { "\n" } else { "" };
                let new_line = line
                    .replace("Voices_ru0", target_token)
                    .replace("Voices_en0", target_token);
                format!("{}{}", new_line, newline)
            } else {
                segment.to_string()
            }
        })
        .collect();

    if replaced {
        Ok(updated)
    } else {
        Err("sResourceArchiveList2 not found in Skyrim.ini".into())
    }
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
    let res = drive
        .download_file(id, mime, out_path.to_str().unwrap(), app)
        .await;
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

    app.emit(
        "update:progress",
        UpdateProgress {
            status: UpdateStatus::DownloadStarted as u8,
        },
    )
    .ok();

    if let Some((id, _, _)) = files.iter().find(|(_, name, _)| name == "version") {
        let tmp = base_dir().join(REMOTE_VERSION_FILE_NAME);
        drive
            .download_file(id, MimeType::Txt, tmp.to_str().unwrap(), app.clone())
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
    let (zip_id, _, _) = files
        .iter()
        .find(|(_, _, mime)| *mime == "application/x-zip-compressed")
        .expect("zip not found");

    let zip_path = base_dir().join(LOCAL_UPDATE_FILE_NAME);
    app.emit(
        "update:progress",
        UpdateProgress {
            status: UpdateStatus::DownloadStarted as u8,
        },
    )
    .ok();
    let _ = drive
        .download_file(
            zip_id,
            MimeType::OctetStream,
            zip_path.to_str().unwrap(),
            app.clone(),
        )
        .await;

    app.emit(
        "update:progress",
        UpdateProgress {
            status: UpdateStatus::DownloadFinished as u8,
        },
    )
    .ok();

    let patch_dir = base_dir().join("mods").join("RFAD_PATCH");
    if patch_dir.exists() {
        fs::remove_dir_all(&patch_dir).unwrap();
    }
    fs::create_dir_all(&patch_dir).unwrap();

    let zip_file = File::open(&zip_path).unwrap();
    let archive = ZipArchive::new(zip_file).unwrap();
    app.emit(
        "update:progress",
        UpdateProgress {
            status: UpdateStatus::UnpackStarted as u8,
        },
    )
    .ok();
    unpack(archive, patch_dir.clone(), &app).await;
    app.emit(
        "update:progress",
        UpdateProgress {
            status: UpdateStatus::UnpackFinished as u8,
        },
    )
    .ok();

    let new_list = remove_whitespace(&new_load_order().await.unwrap_or_default());
    app.emit(
        "update:progress",
        UpdateProgress {
            status: UpdateStatus::LoadOrderUpdateStarted as u8,
        },
    )
    .ok();

    update_modlist();
    if !new_list.is_empty() {
        let plugins_txt = profile_dir().join("plugins.txt");
        let loadorder_txt = profile_dir().join("loadorder.txt");
        update_order(&plugins_txt, &new_list, "*Requiem for the Indifferent.esp")
            .await
            .expect("Error updating plugins.txt");
        update_order(&loadorder_txt, &new_list, "Requiem for the Indifferent.esp")
            .await
            .expect("Error updating loadorder.txt");
    }

    app.emit(
        "update:progress",
        UpdateProgress {
            status: UpdateStatus::LoadOrderUpdateFinished as u8,
        },
    )
    .ok();

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

// #[tauri::command]
// async fn load_json_mods(app: AppHandle) -> String {
//     let drive = gdrive::GoogleDriveClient::new().await;
//     let files = drive.list_files(FOLDER_ID).await;
//
//     if let Some((id, _, _)) = files.iter().find(|(_, name, _)| name == MODS_JSON_FILE_NAME) {
//         let tmp = base_dir().join(MODS_JSON_FILE_NAME);
//         drive.download_file(id, MimeType::Json, tmp.to_str().unwrap(), app.clone())
//             .await
//             .ok();
//         let json = fs::read_to_string(&tmp).unwrap_or_else(|_| "[]".into());
//         let _ = fs::remove_file(&tmp);
//         json
//     } else {
//         "[]".into()
//     }
// }

#[tauri::command]
async fn load_json_patches(app: AppHandle) -> String {
    let drive = gdrive::GoogleDriveClient::new().await;
    let files = drive.list_files(FOLDER_ID).await;

    if let Some((id, _, _)) = files
        .iter()
        .find(|(_, name, _)| name == PATCHES_JSON_FILE_NAME)
    {
        let tmp = base_dir().join(PATCHES_JSON_FILE_NAME);
        drive
            .download_file(id, MimeType::Json, tmp.to_str().unwrap(), app.clone())
            .await
            .ok();
        let json = fs::read_to_string(&tmp).unwrap_or_else(|_| "[]".into());
        let _ = fs::remove_file(&tmp);
        json
    } else {
        "[]".into()
    }
}

#[tauri::command]
async fn update_launcher(download_link: String) -> Result<String, String> {
    let exe_path = exe_dir().join("rfad-launcher.exe");
    if exe_path.exists() {
        fs::rename(exe_path, exe_dir().join("old-launcher.exe")).expect("Failed to rename file");
    }

    let response = reqwest::get(&download_link)
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let dest_path = exe_dir().join("rfad-launcher.exe");

    let mut file = tokio::fs::File::create(&dest_path)
        .await
        .map_err(|e| format!("Failed to create file '{:?}': {}", dest_path, e))?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Error while downloading: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Error writing to file: {}", e))?;
    }

    write_log("update_launcher finished download");

    Ok("Download complete!".to_string())
}

#[tauri::command]
fn start_new_launcher() {
    let new_path = exe_dir().join("rfad-launcher.exe");
    write_log(&format!(
        "Attempting to start new launcher: {}",
        new_path.display()
    ));

    match std::process::Command::new(&new_path).spawn() {
        Ok(_) => write_log("New launcher started successfully"),
        Err(e) => write_log(&format!("Failed to start new launcher: {}", e)),
    }

    std::process::exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(args: Vec<String>) {
    let old_launcher_path = exe_dir().join("old-launcher.exe");
    if old_launcher_path.exists() {
        fs::remove_file(old_launcher_path).expect("Failed to remove old launcher");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            download,
            get_local_version,
            get_remote_version,
            update,
            start_game,
            open_explorer,
            open_mo2,
            is_path_exist,
            get_framerate_limit,
            get_voice_locale,
            update_game_settings,
            load_json_patches,
            update_launcher,
            exe_dir,
            start_new_launcher
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
