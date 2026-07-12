#[derive(Clone, Copy, Debug)]
pub enum RemoteStatus {
    NoDir,
    NoPatch,
    NetError,
    AuthError,
    DriveError,
}

impl RemoteStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RemoteStatus::NoDir => "NO_DIR",
            RemoteStatus::NoPatch => "NO_PATCH",
            RemoteStatus::NetError => "NET_ERROR",
            RemoteStatus::AuthError => "AUTH_ERROR",
            RemoteStatus::DriveError => "DRIVE_ERROR",
        }
    }
}

impl From<RemoteStatus> for String {
    fn from(s: RemoteStatus) -> String {
        s.as_str().to_string()
    }
}
