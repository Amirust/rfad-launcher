//! Launcher self-update with signature verification.
//!
//! Trust chain:
//!   1. `lversions.json` + `lversions.json.sig` are fetched from GitHub; the manifest
//!      signature is checked against the public key embedded in this binary.
//!   2. The manifest version must be strictly greater than the running version
//!      (protects against rollback to an old, legitimately signed build).
//!   3. The exe is downloaded to a temp file; its own `.sig` is verified and its
//!      SHA-256 must match the one in the signed manifest.
//!   4. Only then the running exe is swapped.
//!
//! Signatures are minisign (as produced by `tauri signer sign`): the `.sig` file is
//! base64 of the minisign signature text.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use futures::StreamExt;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::SystemTime};
use tauri::AppHandle;
use tokio::io::AsyncWriteExt;

use crate::write_log;

const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/Amirust/rfad-launcher/main/lversions.json";
/// Only release assets of this repo are accepted as download sources.
const ALLOWED_DOWNLOAD_PREFIX: &str = "https://github.com/Amirust/rfad-launcher/releases/download/";
const UPDATE_PUBLIC_KEY: &str = include_str!("../keys/update.pub");

const EXE_NAME: &str = "rfad-launcher.exe";
const NEW_EXE_NAME: &str = "rfad-launcher.new";
const OLD_EXE_NAME: &str = "old-launcher.exe";

#[derive(Deserialize)]
struct LauncherManifest {
    version: String,
    #[serde(rename = "downloadUrl")]
    download_url: String,
    sha256: String,
}

fn public_key(key_file: &str) -> Result<PublicKey, String> {
    let decoded = B64
        .decode(key_file.trim())
        .map_err(|e| format!("bad embedded public key (base64): {}", e))?;
    let text = String::from_utf8(decoded).map_err(|e| format!("bad embedded public key: {}", e))?;
    PublicKey::decode(&text).map_err(|e| format!("bad embedded public key: {}", e))
}

/// Verifies `data` against a tauri/minisign `.sig` file content.
fn verify(data: &[u8], sig_file: &[u8]) -> Result<(), String> {
    verify_with(UPDATE_PUBLIC_KEY, data, sig_file)
}

fn verify_with(key_file: &str, data: &[u8], sig_file: &[u8]) -> Result<(), String> {
    let sig_text = B64
        .decode(String::from_utf8_lossy(sig_file).trim())
        .map_err(|e| format!("signature is not valid base64: {}", e))?;
    let sig_text = String::from_utf8(sig_text).map_err(|e| format!("bad signature: {}", e))?;
    let signature = Signature::decode(&sig_text).map_err(|e| format!("bad signature: {}", e))?;
    public_key(key_file)?
        .verify(data, &signature, false)
        .map_err(|e| format!("SIGNATURE MISMATCH: {}", e))
}

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|e| format!("request to {} failed: {}", url, e))?;
    if !resp.status().is_success() {
        return Err(format!("{} returned {}", url, resp.status()));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("reading {} failed: {}", url, e))
}

/// Fetches the manifest, verifies its signature and returns it only if it is newer.
async fn fetch_newer_manifest(
    app: &AppHandle,
    client: &reqwest::Client,
) -> Result<Option<(LauncherManifest, Version)>, String> {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let manifest = fetch_bytes(client, &format!("{}?t={}", MANIFEST_URL, ts)).await?;
    let sig = fetch_bytes(client, &format!("{}.sig?t={}", MANIFEST_URL, ts)).await?;

    verify(&manifest, &sig).map_err(|e| format!("manifest: {}", e))?;

    let manifest: LauncherManifest =
        serde_json::from_slice(&manifest).map_err(|e| format!("manifest parse error: {}", e))?;
    let remote = Version::parse(manifest.version.trim())
        .map_err(|e| format!("manifest version '{}' invalid: {}", manifest.version, e))?;
    let current = app.package_info().version.clone();

    if remote > current {
        Ok(Some((manifest, remote)))
    } else {
        Ok(None)
    }
}

/// Returns the new version string if a verified, newer launcher is available.
#[tauri::command]
pub async fn check_launcher_update(app: AppHandle) -> Result<Option<String>, String> {
    let client = reqwest::Client::new();
    match fetch_newer_manifest(&app, &client).await {
        Ok(found) => Ok(found.map(|(_, v)| v.to_string())),
        Err(e) => {
            write_log(&format!("check_launcher_update: {}", e));
            Err(e)
        }
    }
}

/// Downloads, verifies and installs the new launcher. The running exe is only
/// replaced after every check has passed.
#[tauri::command]
pub async fn update_launcher(app: AppHandle) -> Result<String, String> {
    let result = do_update(&app).await;
    if let Err(e) = &result {
        write_log(&format!("update_launcher: ABORTED: {}", e));
        let _ = fs::remove_file(crate::exe_dir().join(NEW_EXE_NAME));
    }
    result
}

async fn do_update(app: &AppHandle) -> Result<String, String> {
    let client = reqwest::Client::new();
    // Re-fetch and re-verify here: never trust data passed from the frontend.
    let (manifest, version) = fetch_newer_manifest(app, &client)
        .await?
        .ok_or("no newer verified version available")?;

    if !manifest.download_url.starts_with(ALLOWED_DOWNLOAD_PREFIX) {
        return Err(format!("download url not allowed: {}", manifest.download_url));
    }

    write_log(&format!("update_launcher: downloading {} ({})", version, manifest.download_url));

    let dir = crate::exe_dir();
    let new_path = dir.join(NEW_EXE_NAME);
    let hash = download_to(&client, &manifest.download_url, &new_path).await?;

    if !hash.eq_ignore_ascii_case(manifest.sha256.trim()) {
        return Err(format!("sha256 mismatch: got {}, manifest says {}", hash, manifest.sha256));
    }

    let exe_sig = fetch_bytes(&client, &format!("{}.sig", manifest.download_url)).await?;
    let exe_bytes = fs::read(&new_path).map_err(|e| format!("cannot read downloaded exe: {}", e))?;
    verify(&exe_bytes, &exe_sig).map_err(|e| format!("exe: {}", e))?;
    drop(exe_bytes);

    write_log("update_launcher: signature and hash OK, swapping executables");
    swap_executables(&dir.join(EXE_NAME), &dir.join(OLD_EXE_NAME), &new_path)?;
    write_log("update_launcher: finished");
    Ok(format!("Updated to {}", version))
}

/// Streams `url` into `dest`, returning the lowercase hex SHA-256 of the content.
async fn download_to(client: &reqwest::Client, url: &str, dest: &Path) -> Result<String, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("server returned {}", resp.status()));
    }

    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("cannot create {:?}: {}", dest, e))?;
    let mut hasher = Sha256::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("download error: {}", e))?;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("write error: {}", e))?;
    }
    file.flush().await.map_err(|e| format!("write error: {}", e))?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn swap_executables(current: &Path, old: &Path, new: &Path) -> Result<(), String> {
    if old.exists() {
        fs::remove_file(old).map_err(|e| format!("cannot remove {:?}: {}", old, e))?;
    }
    let had_current = current.exists();
    if had_current {
        fs::rename(current, old).map_err(|e| format!("cannot move current exe aside: {}", e))?;
    }
    if let Err(e) = fs::rename(new, current) {
        if had_current {
            let _ = fs::rename(old, current); // restore working launcher
        }
        return Err(format!("cannot install new exe: {}", e));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Throwaway key pair, generated only for these tests.
    const TEST_PUB: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IERBMEQ1OTM5MDg3NzdBRTcKUldUbmVuY0lPVmtOMnN3ZTZLTnd2UGVETTJ3Z3Y2U05MZHJra09NNDVVSTF3SUxoZGwyaURUVUgK";
    const TEST_SIG: &[u8] = b"dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVUbmVuY0lPVmtOMmhVZERza0FacVZNZ2YraW84NjBCUHpaWWE1UnVBNTVGdmQ3TzdpSkp3Ymlld3F1QUZLNFc3dUJvWFBzZVNzclIyZE5rYnhybXduT29rbDhlWmdNRVFRPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk1MjY5MDQ0CWZpbGU6ZGF0YS5iaW4KdGNnb3VGT2tlYUNRcHdub1pHUXA5Z1YyaXoyM1NRK0k0cUhtMGRsSzBzaUVTaG53RUNzQ2ZQcVI1MWFwSlNBZy84V1RZV3Q5V1B3ektLK1ZhNjFqQ0E9PQo=";

    #[test]
    fn embedded_key_parses() {
        public_key(UPDATE_PUBLIC_KEY).unwrap();
    }

    #[test]
    fn valid_signature_passes() {
        verify_with(TEST_PUB, b"hello rfad", TEST_SIG).unwrap();
    }

    #[test]
    fn tampered_data_fails() {
        assert!(verify_with(TEST_PUB, b"hello rfaD", TEST_SIG).is_err());
    }

    #[test]
    fn foreign_key_fails() {
        assert!(verify(b"hello rfad", TEST_SIG).is_err());
    }

    #[test]
    fn garbage_signature_fails() {
        assert!(verify_with(TEST_PUB, b"hello rfad", b"not a signature").is_err());
    }
}
