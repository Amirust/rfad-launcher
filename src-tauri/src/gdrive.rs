#[path = "./events.rs"]
mod events;

pub use events::DownloadProgress;

use futures::StreamExt;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::utils::mime_type::MimeType;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

const SERVICE_ACCOUNT_JSON: &str = env!(
    "RFAD_GDRIVE_SA_KEY",
    "RFAD_GDRIVE_SA_KEY is not set — put the service-account JSON in src-tauri/.cargo/config.toml or export it before building"
);
const DRIVE_API: &str = "https://www.googleapis.com/drive/v3";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";

pub struct GoogleDriveClient {
    client: reqwest::Client,
    token: Option<String>,
}

// Only the fields we need from the service-account JSON.
#[derive(Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    private_key_id: String,
    token_uri: String,
}

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    exp: u64,
    iat: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
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

async fn fetch_access_token(client: &reqwest::Client) -> Option<String> {
    let key: ServiceAccountKey = match serde_json::from_str(SERVICE_ACCOUNT_JSON) {
        Ok(k) => k,
        Err(e) => {
            println!("Invalid service account JSON: {:?}", e);
            return None;
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    let claims = Claims {
        iss: &key.client_email,
        scope: SCOPE,
        aud: &key.token_uri,
        exp: now + 3600,
        iat: now,
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key.private_key_id.clone());
    let encoding_key = match EncodingKey::from_rsa_pem(key.private_key.as_bytes()) {
        Ok(k) => k,
        Err(e) => {
            println!("Invalid service account private key: {:?}", e);
            return None;
        }
    };
    let jwt = match encode(&header, &claims, &encoding_key) {
        Ok(j) => j,
        Err(e) => {
            println!("Failed to sign JWT: {:?}", e);
            return None;
        }
    };

    let response = client
        .post(&key.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str()),
        ])
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        println!("Token request failed: {} {}", status, body);
        return None;
    }

    response
        .json::<TokenResponse>()
        .await
        .ok()
        .map(|t| t.access_token)
}

impl GoogleDriveClient {
    pub async fn new() -> Self {
        let client = reqwest::Client::new();
        let token = fetch_access_token(&client).await;
        Self { client, token }
    }

    pub async fn list_files(&self, folder_id: &str) -> Vec<(String, String, String)> {
        let token = match &self.token {
            Some(t) => t,
            None => {
                println!("Error fetching files: no access token");
                return vec![];
            }
        };

        let q = format!("'{}' in parents", folder_id);
        let result = self
            .client
            .get(format!("{}/files", DRIVE_API))
            .bearer_auth(token)
            .query(&[
                ("q", q.as_str()),
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
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| "no access token".to_string())?;

        let url = match mime_type {
            // Native Google formats (Docs) must be exported.
            MimeType::Txt => format!(
                "{}/files/{}/export?mimeType=text/plain",
                DRIVE_API, file_id
            ),
            // Regular binary files are streamed with alt=media.
            _ => format!("{}/files/{}?alt=media", DRIVE_API, file_id),
        };

        let response = self
            .client
            .get(url)
            .bearer_auth(token)
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
        let token = self.token.as_ref().ok_or(())?;

        let url = format!("{}/files/{}/export?mimeType=text/plain", DRIVE_API, file_id);

        let response = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| ())?;
        if !response.status().is_success() {
            return Err(());
        }

        response.text().await.map_err(|_| ())
    }
}
