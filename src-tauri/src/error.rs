use serde::Serialize;
use std::fmt::{Display, Formatter};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: &str, message_key: &str) -> Self {
        Self {
            code: code.to_owned(),
            message_key: message_key.to_owned(),
            detail: None,
        }
    }

    pub fn with_detail(code: &str, message_key: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message_key: message_key.to_owned(),
            detail: Some(detail.into()),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(formatter, "{}: {detail}", self.message_key),
            None => formatter.write_str(&self.message_key),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        Self::with_detail(
            "STORAGE_ERROR",
            "error.storageOperationFailed",
            error.to_string(),
        )
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::with_detail("IO_ERROR", "error.fileOperationFailed", error.to_string())
    }
}

impl From<image::ImageError> for AppError {
    fn from(error: image::ImageError) -> Self {
        Self::with_detail(
            "IMAGE_ERROR",
            "error.imageCouldNotBeSaved",
            error.to_string(),
        )
    }
}

impl From<tauri::Error> for AppError {
    fn from(error: tauri::Error) -> Self {
        Self::with_detail(
            "WINDOW_ERROR",
            "error.windowOperationFailed",
            error.to_string(),
        )
    }
}
