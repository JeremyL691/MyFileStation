use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    File,
    Directory,
    Text,
    Image,
}

impl ItemKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Text => "text",
            Self::Image => "image",
        }
    }

    pub fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "file" => Ok(Self::File),
            "directory" => Ok(Self::Directory),
            "text" => Ok(Self::Text),
            "image" => Ok(Self::Image),
            _ => Err(rusqlite::Error::InvalidColumnType(
                0,
                "kind".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSource {
    External,
    Generated,
}

impl ItemSource {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Generated => "generated",
        }
    }

    pub fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "external" => Ok(Self::External),
            "generated" => Ok(Self::Generated),
            _ => Err(rusqlite::Error::InvalidColumnType(
                0,
                "source".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub id: String,
    pub kind: ItemKind,
    pub source: ItemSource,
    pub path: String,
    pub display_name: String,
    pub is_pinned: bool,
    pub added_at: i64,
    pub is_available: bool,
    pub thumbnail_url: Option<String>,
    #[serde(skip)]
    pub thumbnail_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DockSide {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSetting {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LanguageSetting {
    ZhCn,
    En,
    #[default]
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub dock_side: DockSide,
    pub remove_after_drag_out: bool,
    pub cleanup_temp_on_exit: bool,
    pub autostart: bool,
    pub hotkey: String,
    pub theme: ThemeSetting,
    pub language: LanguageSetting,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dock_side: DockSide::Right,
            remove_after_drag_out: true,
            cleanup_temp_on_exit: true,
            autostart: false,
            hotkey: "Ctrl+Alt+Space".to_owned(),
            theme: ThemeSetting::System,
            language: LanguageSetting::System,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedItems {
    pub items: Vec<ShelfItem>,
    pub duplicate_count: usize,
    pub invalid_count: usize,
}
