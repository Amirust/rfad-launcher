#[path = "./events.rs"]
mod events;

pub use events::DownloadProgress;

use futures::StreamExt;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
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

#[derive(Clone, Copy, Debug)]
pub enum DriveError {
    Network,
    Auth,
    Other,
}

pub struct GoogleDriveClient {
    client: reqwest::Client,
    token: Result<String, DriveError>,
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
    #[serde(default)]
    expires_in: u64,
}

#[derive(Clone)]
struct CachedToken {
    token: String,
    /// Unix seconds at which the token expires.
    expires_at: u64,
}

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static TOKEN_CACHE: OnceLock<Mutex<Option<CachedToken>>> = OnceLock::new();

fn http_client() -> reqwest::Client {
    HTTP_CLIENT.get_or_init(reqwest::Client::new).clone()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn get_access_token(client: &reqwest::Client) -> Result<String, DriveError> {
    let cache = TOKEN_CACHE.get_or_init(|| Mutex::new(None));

    {
        let guard = cache.lock().unwrap();
        if let Some(c) = guard.as_ref() {
            if c.expires_at > now_secs() + 60 {
                return Ok(c.token.clone());
            }
        }
    }

    let fresh = fetch_access_token(client).await?;
    let token = fresh.token.clone();
    *cache.lock().unwrap() = Some(fresh);
    Ok(token)
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

async fn fetch_access_token(client: &reqwest::Client) -> Result<CachedToken, DriveError> {
    let key: ServiceAccountKey = match serde_json::from_str(SERVICE_ACCOUNT_JSON) {
        Ok(k) => k,
        Err(e) => {
            crate::write_log(&format!("gdrive: invalid service account JSON: {:?}", e));
            return Err(DriveError::Other);
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DriveError::Other)?
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
            crate::write_log(&format!("gdrive: invalid service account private key: {:?}", e));
            return Err(DriveError::Other);
        }
    };
    let jwt = match encode(&header, &claims, &encoding_key) {
        Ok(j) => j,
        Err(e) => {
            crate::write_log(&format!("gdrive: failed to sign JWT: {:?}", e));
            return Err(DriveError::Other);
        }
    };

    let response = match client
        .post(&key.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str()),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            crate::write_log(&format!("gdrive: token request network error (Google unreachable): {}", e));
            return Err(DriveError::Network);
        }
    };

    // We reached Google but it rejected the credentials = key/account problem.
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        crate::write_log(&format!("gdrive: token request rejected (auth): {} {}", status, body));
        return Err(DriveError::Auth);
    }

    match response.json::<TokenResponse>().await {
        Ok(t) => {
            let ttl = if t.expires_in > 0 { t.expires_in } else { 3600 };
            crate::write_log(&format!("gdrive: access token obtained (ttl {}s)", ttl));
            Ok(CachedToken {
                token: t.access_token,
                expires_at: now + ttl,
            })
        }
        Err(e) => {
            crate::write_log(&format!("gdrive: failed to parse token response: {:?}", e));
            Err(DriveError::Other)
        }
    }
}

impl GoogleDriveClient {
    pub async fn new() -> Self {
        let client = http_client();
        let token = get_access_token(&client).await;
        Self { client, token }
    }

    pub fn auth_status(&self) -> Result<(), DriveError> {
        self.token.as_ref().map(|_| ()).map_err(|e| *e)
    }

    pub async fn list_files(&self, folder_id: &str) -> Vec<(String, String, String)> {
        let token = match &self.token {
            Ok(t) => t,
            Err(_) => {
                crate::write_log("gdrive: list_files aborted — no access token");
                return vec![];
            }
        };

        let q = format!("'{}' in parents", folder_id);
        crate::write_log(&format!("gdrive: listing files in folder {}", folder_id));
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
                crate::write_log(&format!("gdrive: list_files request failed: {:?}", e));
                return vec![];
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            crate::write_log(&format!("gdrive: list_files HTTP error: {} {}", status, body));
            return vec![];
        }

        match response.json::<FileList>().await {
            Ok(list) => {
                let files: Vec<_> = list
                    .files
                    .into_iter()
                    .map(|f| (f.id, f.name, f.mime_type))
                    .collect();
                crate::write_log(&format!("gdrive: list_files returned {} files", files.len()));
                files
            }
            Err(e) => {
                crate::write_log(&format!("gdrive: failed to parse file list: {:?}", e));
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
        let token = self.token.as_ref().map_err(|_| {
            crate::write_log("gdrive: download aborted — no access token");
            "no access token".to_string()
        })?;

        let url = match mime_type {
            // Native Google formats (Docs) must be exported.
            MimeType::Txt => format!(
                "{}/files/{}/export?mimeType=text/plain",
                DRIVE_API, file_id
            ),
            // Regular binary files are streamed with alt=media.
            _ => format!("{}/files/{}?alt=media", DRIVE_API, file_id),
        };
        crate::write_log(&format!(
            "gdrive: download start id={} -> {}",
            file_id, output_path
        ));

        let response = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| {
                crate::write_log(&format!("gdrive: download request failed id={}: {}", file_id, e));
                e.to_string()
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            crate::write_log(&format!(
                "gdrive: download HTTP error id={}: {} {}",
                file_id, status, body
            ));
            return Err(format!("Error downloading file: {} {}", status, body));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(output_path).await.map_err(|e| {
            crate::write_log(&format!(
                "gdrive: cannot create output file {}: {}",
                output_path, e
            ));
            e.to_string()
        })?;
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
        crate::write_log(&format!(
            "gdrive: download finished id={} ({} bytes) -> {}",
            file_id, downloaded, output_path
        ));
        Ok(())
    }

    pub async fn load_text(&self, file_id: &str) -> Result<String, ()> {
        let token = self.token.as_ref().map_err(|_| {
            crate::write_log("gdrive: load_text aborted — no access token");
        })?;

        let url = format!("{}/files/{}/export?mimeType=text/plain", DRIVE_API, file_id);

        let response = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| {
                crate::write_log(&format!("gdrive: load_text request failed id={}: {}", file_id, e));
            })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            crate::write_log(&format!(
                "gdrive: load_text HTTP error id={}: {} {}",
                file_id, status, body
            ));
            return Err(());
        }

        response.text().await.map_err(|_| ())
    }
}
