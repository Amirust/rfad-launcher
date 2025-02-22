#[path="./events.rs"]
mod events;

pub use events::DownloadProgress;

use std::io;
use google_drive3::{yup_oauth2, yup_oauth2::ServiceAccountKey, DriveHub, hyper_rustls::HttpsConnector, hyper_util::client::legacy::connect::HttpConnector};
use hyper::body::{Body};
use tauri::AppHandle;
use tauri::utils::mime_type::MimeType;
use tauri::{Emitter};
use http_body_util::{BodyStream};
use tokio::io::AsyncWriteExt;
use futures::prelude::*;
use tokio::io::AsyncReadExt;
use futures::pin_mut;

const SCOPE: &str = "https://www.googleapis.com/auth/drive";

pub struct GoogleDriveClient {
    hub: DriveHub<HttpsConnector<HttpConnector>>,
}

impl GoogleDriveClient {
    pub async fn new() -> Self {
        // HARDCODE BTW
        // From https://github.com/sancshous/rfad_launcher/blob/main/main.py
        let service_account: ServiceAccountKey = ServiceAccountKey {
            key_type: Some("service_account".to_string()),
            project_id: Some("kiberone-422110".to_string()),
            private_key_id: Some("1fde9ff94ad4ebe5b911559ca3ffaa337ef95141".to_string()),
            private_key: "-----BEGIN PRIVATE KEY-----\nMIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQC5L1L8uTKSoXtr\n2SiF/ICNc2vaWm7JwSXKpto1TF7pBJlPxGGZ6BHynHY3R9sBo9WpCzu70r7M/Bvw\noPbOw4+XwHDmNYCXiRNLg66bkp+ziJKcdK0Z1HPPDhb1zL1yrzQIE7MzxChgba/m\n8ZX4mPBUBwpkE2fEdRk0ZhF148NSglwikiqQljG8HtwaOTFFIRSAkFNQUIbv4CdD\n6iaiY8qtD2vxs2w8O999eZM4661iF12JKis94Q88rI8QiYtRBNKf5WDOa/JbiCEU\nz6BZaNwostWqkeXgXT3MFqqSF2rACHFsvgNSi1hNlqN0mbq6vZyzSyarfABz3A3W\nr+5ItihdAgMBAAECggEAKeJ5S637sUyS5M7GKp/014l+oHGJ01o7WP2qJxnx8ZRX\ntMH/LVdfD9exqUk4UMOkpMpkpVPCUgzHqQJPMG7tAG7HWlpJjnyzf4X2LTvZoTrH\nplmBeXEjDHbsXIYFZ3YXN6h1BMVeOIk2mu6TdBnraaX6BK6a7sVpgP+A/YAZgoST\n63N8K48yV8NxsEIgE73yKDNuVVWI7FJPvtAux0f5uVlDyXiwWqTalK50MNicSBDC\nr9RpRQNrABTghxNtnI94D8GFWQfYGrgDfD4hn5F/OSGjfOqsGITYsAE+jrbxDu59\nB8iWuIpxWxHxgks4ApOea6zA4PUD43dQKq/vQu43hQKBgQD+rnuO0ZF0mVesXIz6\nE1FPNBsolnQUeYXWUqjhZ0mQaEg3XU7rNsffJqHitpt6cfkx3UVfqSJ9fPQ8JIJ3\nxilczz0dvy5+fuQPMskvTyzbC5KahIw8uMWaFPtMEmwaVXmvXuRnwN+kh6rfyhdn\njp4WUEpVBcGX+oGX81O1yqohOwKBgQC6JL2noKIpJV9Qa/gqUCfIXsnry+k5Cl+0\nEpr/XSWS/6l2m/Lm5Nw1F1sRbPhtpVYHDELdnw9t692MaojVrgkqFImSD/ZLxHXQ\n6DKEoT7FbvkV4vVJxEDwMQDDTpzLKNn0Sgbh0rDv3Y8dlte9RLAqQtNI0YuOVv+6\nVLjGITXDRwKBgBS8cCL4vTcZJSJLhs71s7EXNP7hASKJonQI1udDWaIAW6DmX/6W\nvz9UDeo/o/kcPoXo1jUruDsvaVNcRaMq50M/PGKnpkl2W2tBX1ASyjwrfQxHroNj\nJ/ObsbpH5bVfMEEvILmx4oOq6CbAdZdg7U4zy1mQ1mphYxvUHAS5M5DxAoGAeweV\nopl1FKTy3oC+QZlA8hpUc1kPCPhmUOqLL4UtNH9uTkq8vQc+1IhfVKElgbLprTbZ\nawmadRiUEh7H2hNxUzLHypZqP6HWDQGrgiXhCzVRxLmBTgQ8t4Rr8Kqgz1Zs2B2l\ndtR+xcs2sGPmq94eYZBRfauiBa5Sz6D3j1yb4DkCgYBUB+RjFPSqNQ1JQQAsxT5p\nKr7QLa8idKCJVUJfR0LCdtT2YNPcNFiGNtbFXflKuVZB9hciuSjxySt/yM++8XJQ\nxzeRSabfaeySCbTrHEpJeEegnAZ9OJhHadsm/HgRmHODqxCVokhoSYn+OxjzXeun\nm8e3JMCsdL6AX+oRz/vRyA==\n-----END PRIVATE KEY-----\n".to_string(),
            client_email: "bookingtest@kiberone-422110.iam.gserviceaccount.com".to_string(),
            client_id: Some("111582459063203248377".to_string()),
            auth_uri: Some("https://accounts.google.com/o/oauth2/auth".to_string()),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: Some("https://www.googleapis.com/oauth2/v1/certs".to_string()),
            client_x509_cert_url: Some("https://www.googleapis.com/robot/v1/metadata/x509/bookingtest%40kiberone-422110.iam.gserviceaccount.com".to_string()),
        };

        let auth = yup_oauth2::ServiceAccountAuthenticator::builder(
            service_account,
        ).build().await.unwrap();

        let client = hyper_util::client::legacy::Client::builder(
            hyper_util::rt::TokioExecutor::new()
        )
            .build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build()
            );

        Self {
            hub: DriveHub::new(client, auth),
        }
    }

    pub async fn list_files(&self, folder_id: &str) -> Vec<(String, String, String)> {
        let result = self
            .hub
            .files()
            .list()
            .q(&format!("'{}' in parents", folder_id))
            .add_scope(SCOPE)
            .doit()
            .await;

        match result {
            Ok((_, file_list)) => {
                if let Some(files) = file_list.files {
                    files
                        .into_iter()
                        .map(|file| (file.id.unwrap_or_default(), file.name.unwrap_or_default(), file.mime_type.unwrap_or_default()))
                        .collect()
                } else {
                    vec![]
                }
            }
            Err(e) => {
                println!("Error fetching files: {:?}", e);
                vec![]
            }
        }
    }

    pub async fn download_file(&self, file_id: &str, mime_type: MimeType, output_path: &str, app: AppHandle) -> Result<(), String> {
        let response = match mime_type {
            MimeType::Txt => {
                self.hub.files()
                    .export(file_id, "text/plain")
                    .param("alt", "media")
                    .add_scope(SCOPE)
                    .doit()
                    .await
                    .expect("Error downloading file")
            }
            _ => {
                let (resp, _) = self.hub.files()
                    .get(file_id)
                    .param("alt", "media")
                    .add_scope(SCOPE)
                    .doit()
                    .await
                    .expect("Error downloading file");
                resp
            }
        };

        let total_size = response.size_hint().lower();
        let mut file = tokio::fs::File::create(output_path).await.map_err(|e| e.to_string())?;
        let start_time = std::time::Instant::now();
        let mut downloaded: u64 = 0;

        let body = response.into_body();
        let stream_of_frames = BodyStream::new(body);
        let stream_of_bytes = stream_of_frames
            .try_filter_map(|frame| async move { Ok(frame.into_data().ok()) })
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err));
        let mut reader = tokio_util::io::StreamReader::new(stream_of_bytes);
        pin_mut!(reader);

        let mut buffer = vec![0u8; 4 * 1024 * 1024];
        loop {
            let bytes_read = AsyncReadExt::read(&mut reader, &mut buffer).await.map_err(|e| e.to_string())?;
            if bytes_read == 0 {
                break;
            }

            downloaded += bytes_read as u64;
            file.write_all(&buffer[..bytes_read]).await.map_err(|e| e.to_string())?;

            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 { (downloaded as f64 / elapsed) as u64 } else { 0 };
            let percentage = if total_size > 0 { (downloaded as f64 / total_size as f64) * 100.0 } else { 0.0 };

            app.emit("download:progress", Some(DownloadProgress{
                download_bytes: downloaded,
                percentage,
                speed_bytes_per_sec: speed,
                file_name: output_path.split("/").last().unwrap_or_default().to_string()
            })).ok();
        }

        file.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
