use crate::models::{
    AppSettings, DockSide, ImportedItems, ItemKind, ItemSource, LanguageSetting, ShelfItem,
    ThemeSetting,
};
use crate::{AppError, AppResult};
use image::ImageFormat;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const DATABASE_VERSION: i64 = 2;
const RETAIN_REMOVED_ARTIFACT_DAYS: i64 = 7;
const MILLIS_PER_DAY: i64 = 86_400_000;
const MAX_CLIPBOARD_TEXT_BYTES: usize = 32 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 16_384;
const MAX_IMAGE_PIXELS: u64 = 40_000_000;

pub struct ShelfStore {
    connection: Connection,
    artifacts_dir: PathBuf,
    thumbnails_dir: PathBuf,
}

impl ShelfStore {
    pub fn open(
        data_dir: PathBuf,
        artifacts_dir: PathBuf,
        thumbnails_dir: PathBuf,
    ) -> AppResult<(Self, AppSettings)> {
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&artifacts_dir)?;
        fs::create_dir_all(&thumbnails_dir)?;

        let database_path = data_dir.join("station.db");
        let existed = database_path.exists();
        let connection = Connection::open(&database_path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;

        let existing_version = connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .map_err(database_error)?;
        if existed {
            let integrity: String = connection
                .query_row("PRAGMA quick_check", [], |row| row.get(0))
                .map_err(database_error)?;
            if integrity != "ok" {
                return Err(AppError::with_detail(
                    "DATABASE_CORRUPT",
                    "error.databaseCorrupt",
                    integrity,
                ));
            }
        }
        if existing_version > DATABASE_VERSION {
            return Err(AppError::with_detail(
                "DATABASE_TOO_NEW",
                "error.databaseFromNewerVersion",
                format!("Database version {existing_version} is newer than this application."),
            ));
        }

        if existed && existing_version > 0 && existing_version < DATABASE_VERSION {
            let backup_path = data_dir.join("station.db.migration-backup");
            if !backup_path.exists() {
                connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
                fs::copy(&database_path, backup_path)?;
            }
        }

        connection.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                settings_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS items (
                id TEXT PRIMARY KEY NOT NULL,
                kind TEXT NOT NULL CHECK (kind IN ('file', 'directory', 'text', 'image')),
                source TEXT NOT NULL CHECK (source IN ('external', 'generated')),
                path TEXT NOT NULL,
                display_name TEXT NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0 CHECK (is_pinned IN (0, 1)),
                added_at INTEGER NOT NULL,
                thumbnail_path TEXT
            );
            CREATE INDEX IF NOT EXISTS items_display_order
                ON items (is_pinned DESC, added_at DESC);
            CREATE TABLE IF NOT EXISTS pending_deletions (
                path TEXT PRIMARY KEY NOT NULL,
                delete_after INTEGER NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0
            );",
        )?;

        let has_thumbnail_column = {
            let mut statement = connection.prepare("PRAGMA table_info(items)")?;
            let names = statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            names.iter().any(|name| name == "thumbnail_path")
        };
        if !has_thumbnail_column {
            connection.execute("ALTER TABLE items ADD COLUMN thumbnail_path TEXT", [])?;
        }

        connection.pragma_update(None, "user_version", DATABASE_VERSION)?;

        let mut store = Self {
            connection,
            artifacts_dir,
            thumbnails_dir,
        };

        let settings = match store.load_saved_settings()? {
            Some(settings) => settings,
            None => {
                let settings = load_legacy_settings();
                store.save_settings(&settings)?;
                settings
            }
        };

        Ok((store, settings))
    }

    pub fn list_items(&self) -> AppResult<Vec<ShelfItem>> {
        let mut statement = self.connection.prepare(
            "SELECT id, kind, source, path, display_name, is_pinned, added_at, thumbnail_path
             FROM items
             ORDER BY is_pinned DESC, added_at DESC, display_name COLLATE NOCASE ASC",
        )?;
        let rows = statement.query_map([], |row| {
            let kind = ItemKind::parse(&row.get::<_, String>(1)?)?;
            let source = ItemSource::parse(&row.get::<_, String>(2)?)?;
            let path = row.get::<_, String>(3)?;
            let thumbnail_path = row.get::<_, Option<String>>(7)?;
            Ok((
                ShelfItem {
                    id: row.get(0)?,
                    kind,
                    source,
                    display_name: row.get(4)?,
                    is_pinned: row.get(5)?,
                    added_at: row.get(6)?,
                    is_available: false,
                    thumbnail_url: None,
                    thumbnail_path: thumbnail_path.clone().map(PathBuf::from),
                    path,
                },
                thumbnail_path,
            ))
        })?;

        let mut items = Vec::new();
        for row in rows {
            let (mut item, thumbnail_path) = row?;
            item.is_available = Path::new(&item.path).try_exists().unwrap_or(false);
            if let Some(path) = thumbnail_path.map(PathBuf::from)
                && is_managed_thumbnail(&self.thumbnails_dir, &path)
            {
                item.thumbnail_url = image_data_url(&path);
            }
            items.push(item);
        }
        Ok(items)
    }

    pub fn import_paths(&mut self, paths: Vec<String>) -> AppResult<ImportedItems> {
        let existing_paths = self
            .connection
            .prepare("SELECT path FROM items WHERE source = 'external'")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut normalized: std::collections::HashSet<String> = existing_paths
            .iter()
            .map(|path| normalize_existing_path(path))
            .collect();

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut items = Vec::new();
        let mut duplicate_count = 0;
        let mut invalid_count = 0;

        for raw_path in paths {
            let path = PathBuf::from(raw_path);
            if !path.is_absolute() {
                invalid_count += 1;
                continue;
            }

            let Ok(path) = dunce::canonicalize(&path) else {
                invalid_count += 1;
                continue;
            };

            let Ok(metadata) = fs::metadata(&path) else {
                invalid_count += 1;
                continue;
            };

            let path_text = path.to_string_lossy().into_owned();
            if !normalized.insert(normalize_path(&path_text)) {
                duplicate_count += 1;
                continue;
            }

            let item = ShelfItem {
                id: Uuid::new_v4().to_string(),
                kind: if metadata.is_dir() {
                    ItemKind::Directory
                } else {
                    ItemKind::File
                },
                source: ItemSource::External,
                display_name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| path_text.clone()),
                is_pinned: false,
                added_at: unix_millis(),
                is_available: true,
                thumbnail_url: None,
                thumbnail_path: None,
                path: path_text,
            };
            insert_item(&transaction, &item)?;
            items.push(item);
        }

        transaction.commit()?;
        Ok(ImportedItems {
            items,
            duplicate_count,
            invalid_count,
        })
    }

    pub fn create_text_item(&mut self, text: &str) -> AppResult<ShelfItem> {
        if text.len() > MAX_CLIPBOARD_TEXT_BYTES {
            return Err(AppError::with_detail(
                "CLIPBOARD_TEXT_TOO_LARGE",
                "error.clipboardTextTooLarge",
                format!("{} bytes", text.len()),
            ));
        }
        if text.trim().is_empty() {
            return Err(AppError::new(
                "CLIPBOARD_TEXT_EMPTY",
                "error.clipboardTextEmpty",
            ));
        }

        let id = Uuid::new_v4().to_string();
        let path = self.artifacts_dir.join(format!("text-{id}.txt"));
        write_new_file(&path, text.as_bytes())?;
        let item = generated_item(id, ItemKind::Text, path);
        if let Err(error) = insert_item(&self.connection, &item) {
            let _ = fs::remove_file(&item.path);
            return Err(error.into());
        }
        Ok(item)
    }

    pub fn create_image_item(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> AppResult<ShelfItem> {
        if width == 0 || height == 0 || width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION
        {
            return Err(AppError::new(
                "IMAGE_DIMENSIONS_INVALID",
                "error.imageDimensionsInvalid",
            ));
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| {
                AppError::new("IMAGE_DIMENSIONS_INVALID", "error.imageDimensionsInvalid")
            })?;
        if rgba.len() != expected {
            return Err(AppError::new(
                "IMAGE_DATA_INVALID",
                "error.imageCouldNotBeSaved",
            ));
        }
        if u64::from(width).saturating_mul(u64::from(height)) > MAX_IMAGE_PIXELS
            || rgba.len() > 160 * 1024 * 1024
        {
            return Err(AppError::new("IMAGE_TOO_LARGE", "error.imageTooLarge"));
        }

        let image = image::RgbaImage::from_raw(width, height, rgba.to_vec())
            .ok_or_else(|| AppError::new("IMAGE_DATA_INVALID", "error.imageCouldNotBeSaved"))?;
        let id = Uuid::new_v4().to_string();
        let path = self.artifacts_dir.join(format!("image-{id}.png"));
        let stage_path = self.artifacts_dir.join(format!("image-{id}.part"));
        let mut writer = std::io::BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&stage_path)?,
        );
        let write_result = (|| -> AppResult<()> {
            image::DynamicImage::ImageRgba8(image).write_to(&mut writer, ImageFormat::Png)?;
            writer.flush()?;
            writer.get_ref().sync_all()?;
            Ok(())
        })();
        drop(writer);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&stage_path);
            return Err(error);
        }
        if let Err(error) = fs::rename(&stage_path, &path) {
            let _ = fs::remove_file(&stage_path);
            return Err(error.into());
        }

        let mut item = generated_item(id, ItemKind::Image, path.clone());
        let thumbnail_path =
            item_thumbnail_url(&self.thumbnails_dir, &item.id, Path::new(&item.path))
                .map(PathBuf::from);
        item.thumbnail_url = thumbnail_path.as_deref().and_then(image_data_url);
        item.thumbnail_path = thumbnail_path;
        if let Err(error) = insert_item(&self.connection, &item) {
            let _ = fs::remove_file(path);
            if let Some(thumbnail_path) = &item.thumbnail_path {
                let _ = fs::remove_file(thumbnail_path);
            }
            return Err(error.into());
        }
        Ok(item)
    }

    pub fn set_pinned(&mut self, item_id: &str, pinned: bool) -> AppResult<ShelfItem> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let found = transaction.execute(
            "UPDATE items SET is_pinned = ?1 WHERE id = ?2",
            params![pinned, item_id],
        )?;
        if found == 0 {
            return Err(AppError::new("ITEM_NOT_FOUND", "error.itemNotFound"));
        }
        let item = query_item(&transaction, item_id)?
            .ok_or_else(|| AppError::new("ITEM_NOT_FOUND", "error.itemNotFound"))?;
        transaction.commit()?;
        Ok(item)
    }

    pub fn remove_items(&mut self, item_ids: &[String], force: bool) -> AppResult<Vec<ShelfItem>> {
        let unique_ids: std::collections::HashSet<&str> =
            item_ids.iter().map(String::as_str).collect();
        let artifacts_dir = self.artifacts_dir.clone();
        let thumbnails_dir = self.thumbnails_dir.clone();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut removed = Vec::new();

        for item_id in unique_ids {
            let Some(item) = query_item(&transaction, item_id)? else {
                continue;
            };
            if item.is_pinned && !force {
                continue;
            }

            transaction.execute("DELETE FROM items WHERE id = ?1", [item_id])?;
            if item.source == ItemSource::Generated
                && is_managed_artifact(&artifacts_dir, Path::new(&item.path))
            {
                transaction.execute(
                    "INSERT INTO pending_deletions (path, delete_after, attempts)
                     VALUES (?1, ?2, 0)
                     ON CONFLICT(path) DO UPDATE SET
                       delete_after = MIN(pending_deletions.delete_after, excluded.delete_after)",
                    params![
                        item.path,
                        unix_millis() + RETAIN_REMOVED_ARTIFACT_DAYS * MILLIS_PER_DAY
                    ],
                )?;
            }
            if let Some(thumbnail) = &item.thumbnail_path
                && is_managed_thumbnail(&thumbnails_dir, thumbnail)
            {
                transaction.execute(
                    "INSERT INTO pending_deletions (path, delete_after, attempts)
                     VALUES (?1, ?2, 0)
                     ON CONFLICT(path) DO UPDATE SET
                       delete_after = MIN(pending_deletions.delete_after, excluded.delete_after)",
                    params![
                        thumbnail.to_string_lossy(),
                        unix_millis() + RETAIN_REMOVED_ARTIFACT_DAYS * MILLIS_PER_DAY
                    ],
                )?;
            }
            removed.push(item);
        }

        transaction.commit()?;
        Ok(removed)
    }

    pub fn unpinned_ids(&self) -> AppResult<Vec<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM items WHERE is_pinned = 0")?;
        Ok(statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn cleanup_unpinned(&mut self) -> AppResult<Vec<ShelfItem>> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM items WHERE is_pinned = 0 AND source = 'generated'")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        self.remove_items(&ids, false)
    }

    pub fn existing_paths(&self, item_ids: &[String]) -> AppResult<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for item_id in item_ids {
            let Some(item) = query_item(&self.connection, item_id)? else {
                continue;
            };
            if item.source == ItemSource::Generated
                && !is_managed_artifact(&self.artifacts_dir, Path::new(&item.path))
            {
                return Err(AppError::new("PATH_INVALID", "error.pathInvalid"));
            }
            if Path::new(&item.path).try_exists().unwrap_or(false) {
                paths.push(PathBuf::from(item.path));
            }
        }
        Ok(paths)
    }

    pub fn existing_path(&self, item_id: &str) -> AppResult<PathBuf> {
        let item = query_item(&self.connection, item_id)?
            .ok_or_else(|| AppError::new("ITEM_NOT_FOUND", "error.itemNotFound"))?;
        if item.source == ItemSource::Generated
            && !is_managed_artifact(&self.artifacts_dir, Path::new(&item.path))
        {
            return Err(AppError::new("PATH_INVALID", "error.pathInvalid"));
        }
        if !Path::new(&item.path).try_exists().unwrap_or(false) {
            return Err(AppError::with_detail(
                "FILE_MISSING",
                "error.fileUnavailable",
                item.display_name,
            ));
        }
        Ok(PathBuf::from(item.path))
    }

    pub fn save_settings(&mut self, settings: &AppSettings) -> AppResult<()> {
        let json = serde_json::to_string(settings).map_err(|error| {
            AppError::with_detail(
                "SETTINGS_ERROR",
                "error.settingsCouldNotSave",
                error.to_string(),
            )
        })?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO settings (id, settings_json) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET settings_json = excluded.settings_json",
            [json],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn pending_deletions(&self) -> AppResult<Vec<(PathBuf, i64, u32)>> {
        let mut statement = self
            .connection
            .prepare("SELECT path, delete_after, attempts FROM pending_deletions")?;
        Ok(statement
            .query_map([], |row| {
                Ok((
                    PathBuf::from(row.get::<_, String>(0)?),
                    row.get::<_, i64>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn clear_pending_deletion(&mut self, path: &Path) -> AppResult<()> {
        self.connection.execute(
            "DELETE FROM pending_deletions WHERE path = ?1",
            [path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    pub fn postpone_pending_deletion(&mut self, path: &Path, attempts: u32) -> AppResult<()> {
        let hours = 2_i64.saturating_pow(attempts.min(14)).min(24 * 30);
        self.connection.execute(
            "UPDATE pending_deletions SET attempts = ?2, delete_after = ?3 WHERE path = ?1",
            params![
                path.to_string_lossy().as_ref(),
                attempts,
                unix_millis() + hours * 3_600_000
            ],
        )?;
        Ok(())
    }

    pub fn collect_expired_artifacts(
        &mut self,
        clipboard_paths: Option<&[String]>,
    ) -> AppResult<usize> {
        let Some(clipboard_paths) = clipboard_paths else {
            return Ok(0);
        };
        let held: std::collections::HashSet<String> = clipboard_paths
            .iter()
            .map(|path| normalize_existing_path(path))
            .collect();
        let active_paths: std::collections::HashSet<String> = self.connection
            .prepare("SELECT path FROM items UNION SELECT thumbnail_path FROM items WHERE thumbnail_path IS NOT NULL")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter().map(|path| normalize_existing_path(path)).collect();
        let now = unix_millis();
        let pending = self.pending_deletions()?;
        let artifacts_dir = self.artifacts_dir.clone();
        let thumbnails_dir = self.thumbnails_dir.clone();
        let mut collected = 0;
        for (path, delete_after, attempts) in pending {
            if delete_after > now {
                continue;
            }
            let normalized = normalize_existing_path(&path.to_string_lossy());
            if held.contains(&normalized) {
                continue;
            }
            if active_paths.contains(&normalized) {
                self.clear_pending_deletion(&path)?;
                continue;
            }
            if !is_managed_path(&artifacts_dir, &thumbnails_dir, &path) {
                log::error!(
                    "Ignoring pending deletion outside managed storage: {}",
                    path.display()
                );
                self.clear_pending_deletion(&path)?;
                continue;
            }
            match fs::remove_file(&path) {
                Ok(()) => {
                    self.clear_pending_deletion(&path)?;
                    collected += 1;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    self.clear_pending_deletion(&path)?;
                }
                Err(error) => {
                    log::warn!("Deferred artifact cleanup for {}: {error}", path.display());
                    self.postpone_pending_deletion(&path, attempts.saturating_add(1))?;
                }
            }
        }
        Ok(collected)
    }

    pub fn reconcile_orphaned_artifacts(&mut self) -> AppResult<usize> {
        let mut queued = 0;
        for directory in [&self.artifacts_dir, &self.thumbnails_dir] {
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                let path = entry.path();
                if !entry.file_type()?.is_file()
                    || !is_managed_path(&self.artifacts_dir, &self.thumbnails_dir, &path)
                {
                    continue;
                }
                let referenced: bool = self.connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM items WHERE path = ?1 OR thumbnail_path = ?1)",
                    [path.to_string_lossy().as_ref()],
                    |row| row.get(0),
                )?;
                if referenced {
                    continue;
                }
                let age_ms = entry
                    .metadata()?
                    .modified()
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .map(|age| age.as_millis().min(i64::MAX as u128) as i64)
                    .unwrap_or(0);
                if age_ms < RETAIN_REMOVED_ARTIFACT_DAYS * MILLIS_PER_DAY {
                    continue;
                }
                self.connection.execute(
                "INSERT OR IGNORE INTO pending_deletions (path, delete_after, attempts) VALUES (?1, ?2, 0)",
                params![path.to_string_lossy(), unix_millis()],
            )?;
                queued += 1;
            }
        }
        Ok(queued)
    }

    fn load_saved_settings(&self) -> AppResult<Option<AppSettings>> {
        let json = self
            .connection
            .query_row(
                "SELECT settings_json FROM settings WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        match json {
            Some(json) => serde_json::from_str(&json).map(Some).map_err(|error| {
                AppError::with_detail(
                    "SETTINGS_INVALID",
                    "error.settingsCouldNotRead",
                    error.to_string(),
                )
            }),
            None => Ok(None),
        }
    }
}

fn insert_item(connection: &Connection, item: &ShelfItem) -> rusqlite::Result<usize> {
    connection.execute(
        "INSERT INTO items (id, kind, source, path, display_name, is_pinned, added_at, thumbnail_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            item.id,
            item.kind.as_str(),
            item.source.as_str(),
            item.path,
            item.display_name,
            item.is_pinned,
            item.added_at,
            item.thumbnail_path.as_ref().map(|path| path.to_string_lossy().into_owned())
        ],
    )
}

fn database_error(error: rusqlite::Error) -> AppError {
    if matches!(&error, rusqlite::Error::SqliteFailure(code, _) if matches!(code.code, rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase))
    {
        AppError::with_detail(
            "DATABASE_CORRUPT",
            "error.databaseCorrupt",
            error.to_string(),
        )
    } else {
        error.into()
    }
}

fn query_item(connection: &Connection, item_id: &str) -> rusqlite::Result<Option<ShelfItem>> {
    connection
        .query_row(
            "SELECT id, kind, source, path, display_name, is_pinned, added_at, thumbnail_path
             FROM items WHERE id = ?1",
            [item_id],
            |row| {
                Ok(ShelfItem {
                    id: row.get(0)?,
                    kind: ItemKind::parse(&row.get::<_, String>(1)?)?,
                    source: ItemSource::parse(&row.get::<_, String>(2)?)?,
                    path: row.get(3)?,
                    display_name: row.get(4)?,
                    is_pinned: row.get(5)?,
                    added_at: row.get(6)?,
                    is_available: false,
                    thumbnail_url: None,
                    thumbnail_path: row.get::<_, Option<String>>(7)?.map(PathBuf::from),
                })
            },
        )
        .optional()
}

fn generated_item(id: String, kind: ItemKind, path: PathBuf) -> ShelfItem {
    let path = path.to_string_lossy().into_owned();
    ShelfItem {
        id,
        kind,
        source: ItemSource::Generated,
        display_name: Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Clipboard item".to_owned()),
        is_pinned: false,
        added_at: unix_millis(),
        is_available: true,
        thumbnail_url: None,
        thumbnail_path: None,
        path,
    }
}

fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn normalize_path(path: &str) -> String {
    let normalized = path.replace('/', "\\");
    let normalized = normalized.trim_end_matches('\\');
    normalized.to_lowercase()
}

fn normalize_existing_path(path: &str) -> String {
    let canonical = dunce::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    normalize_path(&canonical.to_string_lossy())
}

fn write_new_file(path: &Path, contents: &[u8]) -> AppResult<()> {
    let stage_path = path.with_extension("part");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stage_path)?;
    let result = file.write_all(contents).and_then(|_| file.sync_all());
    drop(file);
    if let Err(error) = result.and_then(|_| fs::rename(&stage_path, path)) {
        let _ = fs::remove_file(stage_path);
        return Err(error.into());
    }
    Ok(())
}

fn item_thumbnail_url(directory: &Path, id: &str, path: &Path) -> Option<String> {
    let image = image::open(path).ok()?;
    let resized = image.thumbnail(72, 72);
    let thumb_path = directory.join(format!("{id}.png"));
    resized
        .save_with_format(&thumb_path, ImageFormat::Png)
        .ok()?;
    Some(thumb_path.to_string_lossy().into_owned())
}

fn is_generated_filename(path: &Path) -> bool {
    let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
        return false;
    };
    let extension = path.extension().and_then(|ext| ext.to_str());
    [("text-", "txt"), ("image-", "png")]
        .iter()
        .any(|(prefix, suffix)| {
            name.strip_prefix(prefix)
                .is_some_and(|id| Uuid::parse_str(id).is_ok())
                && (extension == Some(suffix) || extension == Some("part"))
        })
}

fn is_managed_artifact(directory: &Path, path: &Path) -> bool {
    path.parent() == Some(directory) && is_generated_filename(path)
}

fn is_managed_thumbnail(directory: &Path, path: &Path) -> bool {
    path.parent() == Some(directory)
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        && path
            .file_stem()
            .is_some_and(|stem| Uuid::parse_str(stem.to_string_lossy().as_ref()).is_ok())
}

fn is_managed_path(artifacts: &Path, thumbnails: &Path, path: &Path) -> bool {
    is_managed_artifact(artifacts, path) || is_managed_thumbnail(thumbnails, path)
}

fn image_data_url(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() > 2 * 1024 * 1024 {
        return None;
    }
    Some(format!(
        "data:image/png;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
    ))
}

fn load_legacy_settings() -> AppSettings {
    let Some(legacy_root) = std::env::var_os("APPDATA") else {
        return AppSettings::default();
    };
    let path = PathBuf::from(legacy_root)
        .join("MyFileStation")
        .join("settings.json");
    let Ok(contents) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        log::warn!("The legacy settings file could not be read; defaults are in use.");
        return AppSettings::default();
    };

    let defaults = AppSettings::default();
    let dock_side = match value.get("dock_side").and_then(serde_json::Value::as_str) {
        Some("left") => DockSide::Left,
        Some("right") => DockSide::Right,
        Some(_) => {
            log::warn!("The legacy dock setting was invalid; its default is in use.");
            defaults.dock_side
        }
        None => defaults.dock_side,
    };

    AppSettings {
        dock_side,
        remove_after_drag_out: legacy_bool(&value, "remove_after_drag_out")
            .unwrap_or(defaults.remove_after_drag_out),
        cleanup_temp_on_exit: legacy_bool(&value, "cleanup_temp_on_exit")
            .unwrap_or(defaults.cleanup_temp_on_exit),
        autostart: legacy_bool(&value, "autostart").unwrap_or(defaults.autostart),
        hotkey: defaults.hotkey,
        theme: ThemeSetting::System,
        language: LanguageSetting::System,
    }
}

fn legacy_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    match value.get(key) {
        None => None,
        Some(value) => match value.as_bool() {
            Some(enabled) => Some(enabled),
            None => {
                log::warn!("The legacy {key} setting was invalid; its default is in use.");
                None
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_store() -> (TempDir, ShelfStore) {
        let root = TempDir::new().unwrap();
        let (store, _) = ShelfStore::open(
            root.path().join("data"),
            root.path().join("artifacts"),
            root.path().join("thumbs"),
        )
        .unwrap();
        (root, store)
    }

    #[test]
    fn opens_a_transactional_database_and_persists_migrated_or_default_settings() {
        let (_root, store) = open_store();
        let settings = store.load_saved_settings().unwrap().unwrap();
        assert_eq!(settings, load_legacy_settings());
        assert_eq!(store.list_items().unwrap().len(), 0);
    }

    #[test]
    fn imports_external_files_and_deduplicates_windows_paths() {
        let (_root, mut store) = open_store();
        let external = TempDir::new().unwrap();
        let file = external.path().join("report.txt");
        fs::write(&file, "keep").unwrap();
        let original_path = file.to_string_lossy().into_owned();
        let result = store
            .import_paths(vec![original_path.clone(), original_path.to_uppercase()])
            .unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.duplicate_count, 1);
        assert_eq!(result.items[0].source, ItemSource::External);
        assert_eq!(result.items[0].kind, ItemKind::File);
    }

    #[test]
    fn classifies_a_directory_without_copying_its_contents() {
        let (_root, mut store) = open_store();
        let directory = TempDir::new().unwrap();
        fs::write(directory.path().join("child.txt"), "inside").unwrap();
        let result = store
            .import_paths(vec![directory.path().to_string_lossy().into_owned()])
            .unwrap();
        assert_eq!(result.items[0].kind, ItemKind::Directory);
        assert_eq!(result.items[0].source, ItemSource::External);
    }

    #[test]
    fn skips_invalid_missing_and_relative_paths_without_losing_valid_files() {
        let (_root, mut store) = open_store();
        let external = TempDir::new().unwrap();
        let file = external.path().join("survives.txt");
        fs::write(&file, "keep").unwrap();
        let result = store
            .import_paths(vec![
                file.to_string_lossy().into_owned(),
                external
                    .path()
                    .join("missing")
                    .to_string_lossy()
                    .into_owned(),
                "relative.txt".to_owned(),
            ])
            .unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.invalid_count, 2);
        assert!(file.exists());
    }

    #[test]
    fn external_files_are_never_removed_when_entries_are_removed() {
        let (_root, mut store) = open_store();
        let external = TempDir::new().unwrap();
        let file = external.path().join("source.txt");
        fs::write(&file, "original").unwrap();
        let item = store
            .import_paths(vec![file.to_string_lossy().into_owned()])
            .unwrap()
            .items
            .remove(0);
        store.remove_items(&[item.id], false).unwrap();
        assert_eq!(fs::read_to_string(file).unwrap(), "original");
        assert_eq!(store.pending_deletions().unwrap().len(), 0);
    }

    #[test]
    fn pinned_items_are_omitted_from_bulk_removal_and_force_removal_is_explicit() {
        let (_root, mut store) = open_store();
        let item = store.create_text_item("pinned").unwrap();
        store.set_pinned(&item.id, true).unwrap();
        assert!(
            store
                .remove_items(std::slice::from_ref(&item.id), false)
                .unwrap()
                .is_empty()
        );
        assert!(store.set_pinned(&item.id, false).unwrap().id == item.id);
        let removed = store
            .remove_items(std::slice::from_ref(&item.id), false)
            .unwrap();
        assert_eq!(removed.len(), 1);
        assert!(store.pending_deletions().unwrap().len() == 1);
    }

    #[test]
    fn forcing_removal_can_only_queue_managed_files_for_collection() {
        let (_root, mut store) = open_store();
        let item = store.create_text_item("pinned").unwrap();
        store.set_pinned(&item.id, true).unwrap();
        let removed = store
            .remove_items(std::slice::from_ref(&item.id), true)
            .unwrap();
        assert_eq!(removed.len(), 1);
        let pending = store.pending_deletions().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, PathBuf::from(&item.path));
    }

    #[test]
    fn an_expired_artifact_is_retained_while_the_system_file_clipboard_references_it() {
        let (_root, mut store) = open_store();
        let item = store.create_text_item("still in clipboard").unwrap();
        let path = PathBuf::from(&item.path);
        store
            .remove_items(std::slice::from_ref(&item.id), false)
            .unwrap();
        store
            .connection
            .execute(
                "UPDATE pending_deletions SET delete_after = 0 WHERE path = ?1",
                [&item.path],
            )
            .unwrap();

        assert_eq!(
            store
                .collect_expired_artifacts(Some(std::slice::from_ref(&item.path)))
                .unwrap(),
            0
        );
        assert!(path.exists());
        assert_eq!(store.collect_expired_artifacts(Some(&[])).unwrap(), 1);
        assert!(!path.exists());
    }

    #[test]
    fn the_collector_never_deletes_a_path_outside_managed_storage() {
        let (_root, mut store) = open_store();
        let external = TempDir::new().unwrap();
        let path = external.path().join("do-not-delete.txt");
        fs::write(&path, "protected").unwrap();
        store
            .connection
            .execute(
                "INSERT INTO pending_deletions (path, delete_after, attempts) VALUES (?1, 0, 0)",
                [path.to_string_lossy().as_ref()],
            )
            .unwrap();

        assert_eq!(store.collect_expired_artifacts(Some(&[])).unwrap(), 0);
        assert_eq!(fs::read_to_string(path).unwrap(), "protected");
    }

    #[test]
    fn ordinary_clipboard_text_can_be_restored_after_a_restart() {
        let root = TempDir::new().unwrap();
        let data = root.path().join("data");
        let artifacts = root.path().join("artifacts");
        let thumbs = root.path().join("thumbs");
        let (mut store, _) =
            ShelfStore::open(data.clone(), artifacts.clone(), thumbs.clone()).unwrap();
        let expected = store.create_text_item("recover me").unwrap();
        drop(store);
        let (store, _) = ShelfStore::open(data, artifacts, thumbs).unwrap();
        let actual = store.list_items().unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].id, expected.id);
        assert_eq!(fs::read_to_string(&actual[0].path).unwrap(), "recover me");
    }

    #[test]
    fn exit_cleanup_removes_only_unpinned_generated_content() {
        let (_root, mut store) = open_store();
        let ordinary = store.create_text_item("temporary").unwrap();
        let pinned = store.create_text_item("keep").unwrap();
        store.set_pinned(&pinned.id, true).unwrap();
        let external = TempDir::new().unwrap();
        let original = external.path().join("do-not-delete.txt");
        fs::write(&original, "safe").unwrap();
        let file = store
            .import_paths(vec![original.to_string_lossy().into_owned()])
            .unwrap()
            .items
            .remove(0);
        let removed = store.cleanup_unpinned().unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].id, ordinary.id);
        let remaining: Vec<_> = store.list_items().unwrap();
        assert!(remaining.iter().any(|item| item.id == pinned.id));
        assert!(remaining.iter().any(|item| item.id == file.id));
        assert!(original.exists());
    }

    #[test]
    fn saving_and_reopening_settings_preserves_every_user_choice() {
        let root = TempDir::new().unwrap();
        let data = root.path().join("data");
        let artifacts = root.path().join("artifacts");
        let thumbs = root.path().join("thumbs");
        let (mut store, _) =
            ShelfStore::open(data.clone(), artifacts.clone(), thumbs.clone()).unwrap();
        let settings = AppSettings {
            dock_side: DockSide::Left,
            remove_after_drag_out: false,
            cleanup_temp_on_exit: false,
            autostart: true,
            hotkey: "CTRL+SHIFT+SPACE".into(),
            theme: ThemeSetting::Dark,
            language: LanguageSetting::ZhCn,
        };
        store.save_settings(&settings).unwrap();
        drop(store);
        let (store, actual) = ShelfStore::open(data, artifacts, thumbs).unwrap();
        assert_eq!(actual, settings);
        assert!(store.list_items().unwrap().is_empty());
    }

    #[test]
    fn non_positive_or_malformed_image_buffers_are_rejected() {
        let (_root, mut store) = open_store();
        assert!(store.create_image_item(0, 4, &[0; 16]).is_err());
        assert!(store.create_image_item(2, 2, &[0; 15]).is_err());
        assert!(
            store
                .create_image_item(MAX_IMAGE_DIMENSION + 1, 1, &[0; 4])
                .is_err()
        );
        assert!(store.list_items().unwrap().is_empty());
    }

    #[test]
    fn clipboard_images_are_saved_atomically_as_openable_png_files() {
        let (_root, mut store) = open_store();
        let mut image = image::RgbaImage::new(4, 3);
        image.put_pixel(0, 0, image::Rgba([18, 82, 235, 255]));
        let item = store.create_image_item(4, 3, image.as_raw()).unwrap();
        assert_eq!(item.kind, ItemKind::Image);
        assert!(item.path.ends_with(".png"));
        let actual = image::open(&item.path).unwrap();
        assert_eq!((actual.width(), actual.height()), (4, 3));
        assert!(store.list_items().unwrap()[0].thumbnail_url.is_some());
        assert_eq!(store.pending_deletions().unwrap().len(), 0);
    }

    #[test]
    fn an_empty_clipboard_text_item_is_not_added() {
        let (_root, mut store) = open_store();
        assert!(store.create_text_item("  \n  ").is_err());
        assert!(store.list_items().unwrap().is_empty());
    }

    #[test]
    fn failed_database_inserts_leave_no_generated_files_or_thumbnails() {
        let (_root, mut store) = open_store();
        store.connection.execute_batch("CREATE TRIGGER reject_insert BEFORE INSERT ON items BEGIN SELECT RAISE(ABORT, 'simulated write failure'); END;").unwrap();
        assert!(
            store
                .create_text_item("do not leak a partial file")
                .is_err()
        );
        assert!(store.create_image_item(2, 2, &[255; 16]).is_err());
        assert!(store.list_items().unwrap().is_empty());
        assert_eq!(fs::read_dir(&store.artifacts_dir).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&store.thumbnails_dir).unwrap().count(), 0);
    }

    #[test]
    fn managed_filenames_require_the_exact_generated_pattern() {
        let id = Uuid::new_v4();
        assert!(is_generated_filename(Path::new(&format!("text-{id}.txt"))));
        assert!(is_generated_filename(Path::new(&format!(
            "image-{id}.part"
        ))));
        assert!(!is_generated_filename(Path::new(&format!(
            "text-{id}.unowned.txt"
        ))));
        assert!(!is_generated_filename(Path::new(&format!("text-{id}.png"))));
    }

    #[test]
    fn crash_recovery_collects_old_staged_files_and_orphaned_thumbnails_only() {
        let (_root, mut store) = open_store();
        let item = store.create_image_item(2, 2, &[255; 16]).unwrap();
        let orphan = store.thumbnails_dir.join(format!("{}.png", Uuid::new_v4()));
        let staged = store
            .artifacts_dir
            .join(format!("text-{}.part", Uuid::new_v4()));
        let unrelated = store.artifacts_dir.join("user-file.txt");
        let old = SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 60 * 60);
        for path in [&orphan, &staged, &unrelated] {
            fs::write(path, "orphan recovery sample").unwrap();
            fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(old)
                .unwrap();
        }
        assert_eq!(store.reconcile_orphaned_artifacts().unwrap(), 2);
        assert_eq!(store.collect_expired_artifacts(Some(&[])).unwrap(), 2);
        assert!(Path::new(&item.path).exists());
        assert!(item.thumbnail_path.unwrap().exists());
        assert!(unrelated.exists());
        assert!(!orphan.exists());
        assert!(!staged.exists());
    }

    #[test]
    fn the_database_never_silently_resets_after_corruption() {
        let root = TempDir::new().unwrap();
        let data = root.path().join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("station.db"), "preserve this broken database").unwrap();
        let result = ShelfStore::open(data.clone(), data.join("artifacts"), data.join("thumbs"));
        assert!(result.is_err());
        assert_eq!(
            fs::read(data.join("station.db")).unwrap(),
            b"preserve this broken database"
        );
    }

    #[test]
    fn v1_database_is_backed_up_before_thumbnail_schema_migration() {
        let root = TempDir::new().unwrap();
        let data = root.path().join("data");
        let artifacts = root.path().join("artifacts");
        let thumbs = root.path().join("thumbs");
        fs::create_dir_all(&data).unwrap();
        let database = data.join("station.db");
        let connection = Connection::open(&database).unwrap();
        connection.execute_batch(
            "CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK(id = 1), settings_json TEXT NOT NULL);
             CREATE TABLE items (id TEXT PRIMARY KEY, kind TEXT NOT NULL, source TEXT NOT NULL, path TEXT NOT NULL,
               display_name TEXT NOT NULL, is_pinned INTEGER NOT NULL DEFAULT 0, added_at INTEGER NOT NULL);
             CREATE TABLE pending_deletions (path TEXT PRIMARY KEY, delete_after INTEGER NOT NULL, attempts INTEGER NOT NULL DEFAULT 0);
             PRAGMA user_version = 1;",
        ).unwrap();
        drop(connection);

        let (store, _) = ShelfStore::open(data.clone(), artifacts, thumbs).unwrap();
        assert_eq!(
            store
                .connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            2
        );
        let backup = Connection::open(data.join("station.db.migration-backup")).unwrap();
        let old_columns = backup
            .prepare("PRAGMA table_info(items)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(!old_columns.iter().any(|name| name == "thumbnail_path"));
    }

    #[test]
    fn normalization_is_case_insensitive_for_windows_file_system_paths() {
        assert_eq!(
            normalize_path("C:/Folder/FILE.txt"),
            normalize_path("c:\\folder\\file.TXT")
        );
    }

    #[test]
    fn deduplicates_dot_segments_and_extended_windows_paths() {
        let (_root, mut store) = open_store();
        let external = TempDir::new().unwrap();
        fs::create_dir(external.path().join("subfolder")).unwrap();
        let path = external.path().join("中文 report.txt");
        fs::write(&path, "safe").unwrap();
        let alias = external
            .path()
            .join("subfolder")
            .join("..")
            .join("中文 report.txt");
        let extended = fs::canonicalize(&path).unwrap();
        let imported = store
            .import_paths(vec![
                path.to_string_lossy().into_owned(),
                alias.to_string_lossy().into_owned(),
                extended.to_string_lossy().into_owned(),
            ])
            .unwrap();
        assert_eq!(imported.items.len(), 1);
        assert_eq!(imported.duplicate_count, 2);
    }

    #[test]
    fn collector_protects_a_reimported_artifact_with_different_path_casing() {
        let (_root, mut store) = open_store();
        let item = store.create_text_item("keep my new reference").unwrap();
        store
            .remove_items(std::slice::from_ref(&item.id), false)
            .unwrap();
        let imported = store.import_paths(vec![item.path.to_uppercase()]).unwrap();
        assert_eq!(imported.items.len(), 1);
        store
            .connection
            .execute("UPDATE pending_deletions SET delete_after = 0", [])
            .unwrap();
        assert_eq!(store.collect_expired_artifacts(Some(&[])).unwrap(), 0);
        assert!(Path::new(&item.path).exists());
        assert!(store.pending_deletions().unwrap().is_empty());
    }

    #[test]
    fn collector_waits_when_the_clipboard_cannot_be_read() {
        let (_root, mut store) = open_store();
        let item = store
            .create_text_item("keep while clipboard is busy")
            .unwrap();
        store
            .remove_items(std::slice::from_ref(&item.id), false)
            .unwrap();
        store
            .connection
            .execute("UPDATE pending_deletions SET delete_after = 0", [])
            .unwrap();
        assert_eq!(store.collect_expired_artifacts(None).unwrap(), 0);
        assert!(Path::new(&item.path).exists());
    }

    #[test]
    fn missing_external_paths_remain_visible_after_restart() {
        let (root, mut store) = open_store();
        let external = TempDir::new().unwrap();
        let path = external.path().join("later-missing.txt");
        fs::write(&path, "keep entry").unwrap();
        let item = store
            .import_paths(vec![path.to_string_lossy().into_owned()])
            .unwrap()
            .items
            .remove(0);
        fs::remove_file(path).unwrap();
        drop(store);
        let (store, _) = ShelfStore::open(
            root.path().join("data"),
            root.path().join("artifacts"),
            root.path().join("thumbs"),
        )
        .unwrap();
        let items = store.list_items().unwrap();
        assert_eq!(items[0].id, item.id);
        assert!(!items[0].is_available);
        assert_eq!(
            store.existing_path(&item.id).unwrap_err().code,
            "FILE_MISSING"
        );
    }

    #[test]
    fn newer_databases_are_rejected_without_changing_the_version() {
        let root = TempDir::new().unwrap();
        let data = root.path().join("data");
        fs::create_dir_all(&data).unwrap();
        let db = data.join("station.db");
        let connection = Connection::open(&db).unwrap();
        connection
            .pragma_update(None, "user_version", DATABASE_VERSION + 1)
            .unwrap();
        drop(connection);
        assert!(
            ShelfStore::open(
                data,
                root.path().join("artifacts"),
                root.path().join("thumbs")
            )
            .is_err()
        );
        let connection = Connection::open(db).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            DATABASE_VERSION + 1
        );
    }

    #[test]
    #[ignore = "Explicit storage stress and timing check; creates 1,000 external files and 1,000 generated items"]
    fn thousand_item_storage_stress() {
        use std::time::Instant;
        let (root, mut store) = open_store();
        let external = TempDir::new().unwrap();
        let paths: Vec<String> = (0..1000)
            .map(|index| {
                let path = external.path().join(format!("中文 report {index}.txt"));
                fs::write(&path, "external source must survive").unwrap();
                path.to_string_lossy().into_owned()
            })
            .collect();
        let started = Instant::now();
        let imported = store.import_paths(paths.clone()).unwrap();
        println!("Import 1000 external references: {:?}", started.elapsed());
        assert_eq!(imported.items.len(), 1000);
        assert_eq!(
            store.import_paths(paths.clone()).unwrap().duplicate_count,
            1000
        );
        let mut list_times = Vec::new();
        for _ in 0..50 {
            let started = Instant::now();
            assert_eq!(store.list_items().unwrap().len(), 1000);
            list_times.push(started.elapsed());
        }
        list_times.sort();
        println!(
            "List 1000 references P95 (50 samples): {:?}",
            list_times[47]
        );
        store
            .remove_items(
                &imported
                    .items
                    .iter()
                    .map(|item| item.id.clone())
                    .collect::<Vec<_>>(),
                false,
            )
            .unwrap();
        for path in paths {
            assert_eq!(
                fs::read_to_string(path).unwrap(),
                "external source must survive"
            );
        }
        let started = Instant::now();
        for index in 0..1000 {
            let item = if index % 10 == 0 {
                store.create_image_item(2, 2, &[255; 16]).unwrap()
            } else {
                store
                    .create_text_item("generated lifecycle stress")
                    .unwrap()
            };
            store.remove_items(&[item.id], false).unwrap();
        }
        println!(
            "1000 generated create/remove cycles: {:?}",
            started.elapsed()
        );
        assert!(store.list_items().unwrap().is_empty());
        assert_eq!(store.pending_deletions().unwrap().len(), 1100);
        drop(store);
        let (mut store, _) = ShelfStore::open(
            root.path().join("data"),
            root.path().join("artifacts"),
            root.path().join("thumbs"),
        )
        .unwrap();
        store
            .connection
            .execute("UPDATE pending_deletions SET delete_after = 0", [])
            .unwrap();
        assert_eq!(store.collect_expired_artifacts(Some(&[])).unwrap(), 1100);
        assert!(store.pending_deletions().unwrap().is_empty());
        assert_eq!(fs::read_dir(&store.artifacts_dir).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&store.thumbnails_dir).unwrap().count(), 0);
    }
}
