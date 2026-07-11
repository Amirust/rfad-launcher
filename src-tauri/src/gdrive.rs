#[path = "./events.rs"]
mod events;

pub use events::DownloadProgress;

use futures::StreamExt;
use serde::Deserialize;
use tauri::utils::mime_type::MimeType;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

// Injected at compile time from the RFAD_GDRIVE_API_KEY environment variable
const API_KEY: &str = env!(
    "RFAD_GDRIVE_API_KEY",
    "RFAD_GDRIVE_API_KEY is not set — put it in src-tauri/.cargo/config.toml or export it before building"
);
const DRIVE_API: &str = "https://www.googleapis.com/drive/v3";

pub struct GoogleDriveClient {
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct FileEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "mimeType")]
    mime_type: String,
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<FileEntry>,
}

impl GoogleDriveClient {
    pub async fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn list_files(&self, folder_id: &str) -> Vec<(String, String, String)> {
        let q = format!("'{}' in parents", folder_id);
        let result = self
            .client
            .get(format!("{}/files", DRIVE_API))
            .query(&[
                ("q", q.as_str()),
                ("key", API_KEY),
                ("fields", "files(id,name,mimeType)"),
                ("pageSize", "1000"),
            ])
            .send()
            .await;

        let response = match result {
            Ok(resp) => resp,
            Err(e) => {
                println!("Error fetching files: {:?}", e);
                return vec![];
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            println!("Error fetching files: {} {}", status, body);
            return vec![];
        }

        match response.json::<FileList>().await {
            Ok(list) => list
                .files
                .into_iter()
                .map(|f| (f.id, f.name, f.mime_type))
                .collect(),
            Err(e) => {
                println!("Error parsing file list: {:?}", e);
                vec![]
            }
        }
    }

    pub async fn download_file(
        &self,
        file_id: &str,
        mime_type: MimeType,
        output_path: &str,
        app: AppHandle,
    ) -> Result<(), String> {
        let url = match mime_type {
            // Native Google formats (Docs) must be exported.
            MimeType::Txt => format!(
                "{}/files/{}/export?mimeType=text/plain&key={}",
                DRIVE_API, file_id, API_KEY
            ),
            // Regular binary files are streamed with alt=media.
            _ => format!("{}/files/{}?alt=media&key={}", DRIVE_API, file_id, API_KEY),
        };

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Error downloading file: {} {}", status, body));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(output_path)
            .await
            .map_err(|e| e.to_string())?;
        let start_time = std::time::Instant::now();
        let mut downloaded: u64 = 0;
        let mut last_emit: u64 = 0;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;

            // Throttle progress events to avoid flooding the frontend.
            if downloaded - last_emit >= 1024 * 1024 || downloaded == total_size {
                last_emit = downloaded;

                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    (downloaded as f64 / elapsed) as u64
                } else {
                    0
                };
                let percentage = if total_size > 0 {
                    (downloaded as f64 / total_size as f64) * 100.0
                } else {
                    0.0
                };

                app.emit(
                    "download:progress",
                    Some(DownloadProgress {
                        download_bytes: downloaded,
                        percentage,
                        speed_bytes_per_sec: speed,
                        file_name: output_path
                            .split(['/', '\\'])
                            .last()
                            .unwrap_or_default()
                            .to_string(),
                    }),
                )
                .ok();
            }
        }

        file.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn load_text(&self, file_id: &str) -> Result<String, ()> {
        let url = format!(
            "{}/files/{}/export?mimeType=text/plain&key={}",
            DRIVE_API, file_id, API_KEY
        );

        let response = self.client.get(url).send().await.map_err(|_| ())?;
        if !response.status().is_success() {
            return Err(());
        }

        response.text().await.map_err(|_| ())
    }
}
