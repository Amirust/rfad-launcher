use serde::Serialize;

#[derive(Clone, Serialize)]
#[repr(u8)]
pub enum UpdateStatus {
    DownloadStarted = 0,
    DownloadFinished = 1,
    UnpackStarted = 2,
    UnpackFinished = 3,
    LoadOrderUpdateStarted = 4,
    LoadOrderUpdateFinished = 5
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub status: u8
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub file_name: String,
    pub download_bytes: u64,
    pub percentage: f64,
    pub speed_bytes_per_sec: u64
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpackProgress {
    pub percentage: f64,
}
