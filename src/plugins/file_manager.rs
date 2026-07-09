use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode {
    Normal,
    RecentFiles,
    RecentDirs,
}

#[derive(Clone, Copy, Debug)]
pub enum ClipboardOp {
    Copy,
    Cut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDialogAction {
    Rename,
    NewFile,
    NewFolder,
}

pub struct FileManagerPlugin {
    current_dir: RwLock<PathBuf>,
    clipboard: RwLock<Option<(PathBuf, ClipboardOp)>>,
    focus_pane: RwLock<usize>, // 0: Sidebar, 1: Files, 2: Path, 3: Search
    sidebar_selected_idx: RwLock<usize>,
    recent_dirs: RwLock<Vec<PathBuf>>,
    recent_files: RwLock<Vec<PathBuf>>,
    view_mode: RwLock<ViewMode>,
    context_menu_open: RwLock<bool>,
    context_menu_selected_idx: RwLock<usize>,
    show_hidden: RwLock<bool>,
    input_dialog_open: RwLock<bool>,
    input_dialog_title: RwLock<String>,
    input_dialog_buffer: RwLock<String>,
    input_dialog_action: RwLock<InputDialogAction>,
    properties_dialog_open: RwLock<bool>,
    properties_dialog_item_id: RwLock<String>,
    delete_confirm_open: RwLock<bool>,
    delete_confirm_item_id: RwLock<String>,
    delete_confirm_selected_idx: RwLock<usize>, // 0: No, 1: Yes
}

impl FileManagerPlugin {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/fd")));
        Self {
            current_dir: RwLock::new(current_dir.clone()),
            clipboard: RwLock::new(None),
            focus_pane: RwLock::new(1), // Start focus on Files list
            sidebar_selected_idx: RwLock::new(0),
            recent_dirs: RwLock::new(vec![current_dir]),
            recent_files: RwLock::new(Vec::new()),
            view_mode: RwLock::new(ViewMode::Normal),
            context_menu_open: RwLock::new(false),
            context_menu_selected_idx: RwLock::new(0),
            show_hidden: RwLock::new(false),
            input_dialog_open: RwLock::new(false),
            input_dialog_title: RwLock::new(String::new()),
            input_dialog_buffer: RwLock::new(String::new()),
            input_dialog_action: RwLock::new(InputDialogAction::NewFile),
            properties_dialog_open: RwLock::new(false),
            properties_dialog_item_id: RwLock::new(String::new()),
            delete_confirm_open: RwLock::new(false),
            delete_confirm_item_id: RwLock::new(String::new()),
            delete_confirm_selected_idx: RwLock::new(0),
        }
    }

    fn record_recent_dir(&self, path: PathBuf) {
        if let Ok(mut dirs) = self.recent_dirs.write() {
            dirs.retain(|p| p != &path);
            dirs.insert(0, path);
            dirs.truncate(15);
        }
    }

    fn record_recent_file(&self, path: PathBuf) {
        if let Ok(mut files) = self.recent_files.write() {
            files.retain(|p| p != &path);
            files.insert(0, path);
            files.truncate(15);
        }
    }

    fn format_size(&self, size: u64) -> String {
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    fn get_dir_size_str(&self, path: &Path) -> String {
        match fs::read_dir(path) {
            Ok(entries) => {
                let count = entries.flatten().count();
                if count == 1 {
                    "1 item".to_string()
                } else {
                    format!("{} items", count)
                }
            }
            Err(_) => "DIR".to_string(),
        }
    }
}

fn get_file_icon(path: &Path, is_dir: bool) -> &'static str {
    if is_dir {
        "📁"
    } else {
        match path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" => "🖼️",
            "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "📦",
            "pdf" | "epub" => "📕",
            "doc" | "docx" | "odt" => "📝",
            "xls" | "xlsx" | "ods" | "csv" => "📊",
            "ppt" | "pptx" | "odp" => "📈",
            "txt" | "md" | "json" | "toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" => "📄",
            "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "go" | "sh" | "html" | "css" | "java" | "kt" | "swift" | "rb" | "pl" | "php" => "💻",
            "mp3" | "wav" | "flac" | "ogg" | "m4a" => "🎵",
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" => "🎥",
            _ => "📄",
        }
    }
}

pub(crate) fn get_file_type_desc(path: &Path, is_dir: bool) -> String {
    if is_dir {
        "Directory".to_string()
    } else {
        match path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase().as_str() {
            "rs" => "Rust Source File".to_string(),
            "py" => "Python Script".to_string(),
            "js" => "JavaScript File".to_string(),
            "ts" => "TypeScript File".to_string(),
            "sh" => "Shell Script".to_string(),
            "md" => "Markdown Document".to_string(),
            "toml" => "TOML Configuration".to_string(),
            "json" => "JSON Data".to_string(),
            "yaml" | "yml" => "YAML Configuration".to_string(),
            "txt" => "Text Document".to_string(),
            "png" | "jpg" | "jpeg" => "JPEG/PNG Image".to_string(),
            "gif" => "GIF Image".to_string(),
            "zip" | "tar" | "gz" | "rar" | "7z" => "Archive".to_string(),
            "pdf" => "PDF Document".to_string(),
            _ => "File".to_string(),
        }
    }
}

#[cfg(unix)]
pub(crate) fn get_permissions_str(path: &Path) -> String {
    use std::os::unix::fs::MetadataExt;
    if let Ok(meta) = fs::metadata(path) {
        let mode = meta.mode();
        let is_dir = path.is_dir();
        let mut s = String::with_capacity(10);
        s.push(if is_dir { 'd' } else { '-' });
        s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
        s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
        s.push(if mode & 0o100 != 0 { 'x' } else { '-' });
        s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
        s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
        s.push(if mode & 0o010 != 0 { 'x' } else { '-' });
        s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
        s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
        s.push(if mode & 0o001 != 0 { 'x' } else { '-' });
        return s;
    }
    "---------".to_string()
}

#[cfg(not(unix))]
pub(crate) fn get_permissions_str(_path: &Path) -> String {
    "---------".to_string()
}

#[cfg(unix)]
pub(crate) fn get_owner_str(path: &Path) -> String {
    use std::os::unix::fs::MetadataExt;
    if let Ok(meta) = fs::metadata(path) {
        return format!("{}:{}", meta.uid(), meta.gid());
    }
    "unknown".to_string()
}

#[cfg(not(unix))]
pub(crate) fn get_owner_str(_path: &Path) -> String {
    "unknown".to_string()
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(&entry.path(), &dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn paste_item(src: &Path, dst: &Path, op: ClipboardOp) -> std::io::Result<()> {
    if let ClipboardOp::Cut = op {
        if fs::rename(src, dst).is_ok() {
            return Ok(());
        }
    }

    if src.is_dir() {
        copy_dir_all(src, dst)?;
    } else {
        fs::copy(src, dst)?;
    }

    if let ClipboardOp::Cut = op {
        if src.is_dir() {
            fs::remove_dir_all(src)?;
        } else {
            fs::remove_file(src)?;
        }
    }
    Ok(())
}

fn get_disk_space_info(dir: &Path) -> Option<(String, String, String)> {
    let output = std::process::Command::new("df")
        .arg("-h")
        .arg(dir)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.len() >= 2 {
        let cols: Vec<&str> = lines[1].split_whitespace().collect();
        if cols.len() >= 5 {
            return Some((cols[1].to_string(), cols[2].to_string(), cols[4].to_string()));
        }
    }
    None
}

fn count_items(path: &Path) -> Option<usize> {
    fs::read_dir(path).map(|entries| entries.flatten().count()).ok()
}

fn get_sidebar_items() -> Vec<(String, PathBuf, &'static str)> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/fd"));
    let desktop = dirs::desktop_dir().unwrap_or_else(|| {
        let p = home.join("Desktop");
        if p.exists() { p } else { home.join("桌面") }
    });
    let docs = dirs::document_dir().unwrap_or_else(|| {
        let p = home.join("Documents");
        if p.exists() { p } else { home.join("文档") }
    });
    let downloads = dirs::download_dir().unwrap_or_else(|| {
        let p = home.join("Downloads");
        if p.exists() { p } else { home.join("下载") }
    });
    let music = dirs::audio_dir().unwrap_or_else(|| {
        let p = home.join("Music");
        if p.exists() { p } else { home.join("音乐") }
    });
    let pics = dirs::picture_dir().unwrap_or_else(|| {
        let p = home.join("Pictures");
        if p.exists() { p } else { home.join("图片") }
    });
    let videos = dirs::video_dir().unwrap_or_else(|| {
        let p = home.join("Videos");
        if p.exists() { p } else { home.join("视频") }
    });
    let trash = home.join(".local/share/Trash/files");

    let mut items = vec![
        // Places
        ("Home".to_string(), home.clone(), "home"),
        ("Desktop".to_string(), desktop, "desktop"),
        ("Documents".to_string(), docs, "documents"),
        ("Downloads".to_string(), downloads, "downloads"),
        ("Music".to_string(), music, "music"),
        ("Pictures".to_string(), pics, "pictures"),
        ("Videos".to_string(), videos, "videos"),
        ("Trash".to_string(), trash, "trash"),
        // Recently used
        ("Recent Files".to_string(), home.clone(), "recent_files"),
        ("Recent Locations".to_string(), home.clone(), "recent_locations"),
    ];

    // Storage devices
    items.push(("系统根目录".to_string(), PathBuf::from("/"), "drive_root"));

    let user = std::env::var("USER").unwrap_or_else(|_| "fd".to_string());
    let media_paths = [
        format!("/run/media/{}", user),
        format!("/media/{}", user),
        "/media".to_string(),
    ];

    for media_path in &media_paths {
        let path = Path::new(media_path);
        if path.exists() && path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Prevent duplicates
                        if !items.iter().any(|(_, p, _)| p == &entry_path) {
                            let display_name = match name.as_str() {
                                "3CE1463EDAD113DD" => "Basic data partition".to_string(),
                                "005B2FCA24517703" => "664.3 GiB 内置驱动器".to_string(),
                                _ => name,
                            };
                            items.push((display_name, entry_path, "drive_external"));
                        }
                    }
                }
            }
        }
    }

    items
}

impl Plugin for FileManagerPlugin {
    fn id(&self) -> &'static str {
        "file_manager"
    }

    fn name(&self) -> &'static str {
        "File Manager"
    }

    fn description(&self) -> &'static str {
        "Browse and manage files (Dolphin layout)"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let current_dir = match self.current_dir.read() {
            Ok(guard) => guard.clone(),
            Err(_) => return Vec::new(),
        };

        // Record current directory in history
        self.record_recent_dir(current_dir.clone());

        let focus_pane = self.focus_pane.read().map(|b| *b).unwrap_or(1);
        let sidebar_focused = focus_pane == 0;
        let sidebar_selected_idx = self.sidebar_selected_idx.read().map(|i| *i).unwrap_or(0);
        let show_hidden = self.show_hidden.read().map(|b| *b).unwrap_or(false);

        let mut results = Vec::new();
        let mut total_dirs = 0;
        let mut total_files = 0;

        let mode = self.view_mode.read().map(|m| *m).unwrap_or(ViewMode::Normal);

        if mode == ViewMode::RecentFiles || mode == ViewMode::RecentDirs {
            let paths = if mode == ViewMode::RecentFiles {
                self.recent_files.read().unwrap().clone()
            } else {
                self.recent_dirs.read().unwrap().clone()
            };

            let matcher = Matcher::new();
            for path in paths {
                if !path.exists() {
                    continue;
                }
                let filename = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if filename.is_empty() {
                    continue;
                }
                let is_dir = path.is_dir();
                if is_dir {
                    total_dirs += 1;
                } else {
                    total_files += 1;
                }

                let score = if query.is_empty() {
                    Some(0)
                } else {
                    matcher.fuzzy_match(&filename, query).map(|s| s as i64)
                };

                if let Some(score) = score {
                    let meta = fs::metadata(&path).ok();
                    let size_str = if is_dir {
                        "DIR".to_string()
                    } else {
                        meta.as_ref().map(|m| self.format_size(m.len())).unwrap_or_default()
                    };

                    let modified_str = meta
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let datetime: DateTime<Local> = t.into();
                            datetime.format("%Y-%m-%d %H:%M").to_string()
                        })
                        .unwrap_or_default();

                    let icon = get_file_icon(&path, is_dir);
                    let permissions_str = get_permissions_str(&path);
                    let owner_str = get_owner_str(&path);

                    let mut metadata = HashMap::new();
                    metadata.insert("path".to_string(), path.to_string_lossy().to_string());
                    metadata.insert("is_dir".to_string(), is_dir.to_string());
                    metadata.insert("name".to_string(), filename.clone());
                    metadata.insert("icon".to_string(), icon.to_string());
                    metadata.insert("size".to_string(), size_str);
                    metadata.insert("modified".to_string(), modified_str);
                    metadata.insert("permissions".to_string(), permissions_str);
                    metadata.insert("owner".to_string(), owner_str);

                    results.push(SearchResult {
                        id: path.to_string_lossy().to_string(),
                        title: filename,
                        subtitle: Some(path.to_string_lossy().to_string()),
                        score,
                        plugin_id: self.id(),
                        metadata,
                    });
                }
            }
        } else {
            // 1. Add Parent Directory ".." entry if available
            if let Some(parent) = current_dir.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                let mut metadata = HashMap::new();
                metadata.insert("path".to_string(), parent_str.clone());
                metadata.insert("is_dir".to_string(), "true".to_string());
                metadata.insert("is_parent".to_string(), "true".to_string());
                metadata.insert("name".to_string(), "..".to_string());
                metadata.insert("icon".to_string(), "📁".to_string());
                metadata.insert("size".to_string(), "Parent Dir".to_string());
                metadata.insert("modified".to_string(), "".to_string());
                metadata.insert("permissions".to_string(), get_permissions_str(parent));
                metadata.insert("owner".to_string(), get_owner_str(parent));

                let matches_query = if query.is_empty() {
                    true
                } else {
                    Matcher::new().fuzzy_match("..", query).is_some()
                };

                if matches_query {
                    results.push(SearchResult {
                        id: "..".to_string(),
                        title: "..".to_string(),
                        subtitle: Some(format!("Go up to: {}", parent.display())),
                        score: 9999,
                        plugin_id: self.id(),
                        metadata,
                    });
                }
            }

            // 2. Read entries from current_dir
            if let Ok(entries) = fs::read_dir(&current_dir) {
                let matcher = Matcher::new();

                for entry in entries.flatten() {
                    let path = entry.path();
                    let filename = entry.file_name().to_string_lossy().to_string();

                    // Skip hidden files if not show_hidden
                    if filename.starts_with('.') && !show_hidden {
                        continue;
                    }

                    let is_dir = path.is_dir();
                    if is_dir {
                        total_dirs += 1;
                    } else {
                        total_files += 1;
                    }

                    let score = if query.is_empty() {
                        Some(0)
                    } else {
                        matcher.fuzzy_match(&filename, query).map(|s| s as i64)
                    };

                    if let Some(score) = score {
                        let meta = entry.metadata().ok();
                        let size_str = if is_dir {
                            "DIR".to_string()
                        } else {
                            meta.as_ref().map(|m| self.format_size(m.len())).unwrap_or_default()
                        };

                        let modified_str = meta
                            .and_then(|m| m.modified().ok())
                            .map(|t| {
                                let datetime: DateTime<Local> = t.into();
                                datetime.format("%Y-%m-%d %H:%M").to_string()
                            })
                            .unwrap_or_default();

                        let icon = get_file_icon(&path, is_dir);
                        let permissions_str = get_permissions_str(&path);
                        let owner_str = get_owner_str(&path);

                        let mut metadata = HashMap::new();
                        metadata.insert("path".to_string(), path.to_string_lossy().to_string());
                        metadata.insert("is_dir".to_string(), is_dir.to_string());
                        metadata.insert("name".to_string(), filename.clone());
                        metadata.insert("icon".to_string(), icon.to_string());
                        metadata.insert("size".to_string(), size_str);
                        metadata.insert("modified".to_string(), modified_str);
                        metadata.insert("permissions".to_string(), permissions_str);
                        metadata.insert("owner".to_string(), owner_str);

                        results.push(SearchResult {
                            id: path.to_string_lossy().to_string(),
                            title: filename,
                            subtitle: Some(path.to_string_lossy().to_string()),
                            score,
                            plugin_id: self.id(),
                            metadata,
                        });
                    }
                }
            }
        }

        // Sorting:
        // If query is empty: sort directories alphabetically first, then files alphabetically.
        // If query is not empty: sort by score descending.
        if query.is_empty() {
            results.sort_by(|a, b| {
                let a_is_parent = a.metadata.get("is_parent").map(|s| s == "true").unwrap_or(false);
                let b_is_parent = b.metadata.get("is_parent").map(|s| s == "true").unwrap_or(false);

                if a_is_parent {
                    return std::cmp::Ordering::Less;
                }
                if b_is_parent {
                    return std::cmp::Ordering::Greater;
                }

                let a_is_dir = a.metadata.get("is_dir").map(|s| s == "true").unwrap_or(false);
                let b_is_dir = b.metadata.get("is_dir").map(|s| s == "true").unwrap_or(false);

                if a_is_dir != b_is_dir {
                    b_is_dir.cmp(&a_is_dir)
                } else {
                    a.title.to_lowercase().cmp(&b.title.to_lowercase())
                }
            });
        } else {
            results.sort_by(|a, b| b.score.cmp(&a.score));
        }

        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/fd"));
        let desktop = dirs::desktop_dir().unwrap_or_else(|| {
            let p = home.join("Desktop");
            if p.exists() { p } else { home.join("桌面") }
        });
        let docs = dirs::document_dir().unwrap_or_else(|| {
            let p = home.join("Documents");
            if p.exists() { p } else { home.join("文档") }
        });
        let downloads = dirs::download_dir().unwrap_or_else(|| {
            let p = home.join("Downloads");
            if p.exists() { p } else { home.join("下载") }
        });
        let music = dirs::audio_dir().unwrap_or_else(|| {
            let p = home.join("Music");
            if p.exists() { p } else { home.join("音乐") }
        });
        let pics = dirs::picture_dir().unwrap_or_else(|| {
            let p = home.join("Pictures");
            if p.exists() { p } else { home.join("图片") }
        });
        let videos = dirs::video_dir().unwrap_or_else(|| {
            let p = home.join("Videos");
            if p.exists() { p } else { home.join("视频") }
        });
        let trash = home.join(".local/share/Trash/files");

        let home_count = count_items(&home).unwrap_or(0);
        let desktop_count = count_items(&desktop).unwrap_or(0);
        let docs_count = count_items(&docs).unwrap_or(0);
        let downloads_count = count_items(&downloads).unwrap_or(0);
        let music_count = count_items(&music).unwrap_or(0);
        let pics_count = count_items(&pics).unwrap_or(0);
        let videos_count = count_items(&videos).unwrap_or(0);
        let trash_count = count_items(&trash).unwrap_or(0);

        let (total_size, used_size, use_percent) = get_disk_space_info(&current_dir)
            .unwrap_or_else(|| ("-".to_string(), "-".to_string(), "-%".to_string()));

        // Store path and sidebar states inside metadata for rendering
        let current_path_str = current_dir.to_string_lossy().to_string();

        let clip_info = self.clipboard.read().ok().and_then(|g| g.clone());
        let (clip_path_str, clip_op_str) = match clip_info {
            Some((path, op)) => (path.to_string_lossy().to_string(), format!("{:?}", op)),
            None => (String::new(), String::new()),
        };

        if results.is_empty() {
            let mut metadata = HashMap::new();
            metadata.insert("is_dummy".to_string(), "true".to_string());
            results.push(SearchResult {
                id: "dummy_metadata_carrier".to_string(),
                title: "".to_string(),
                subtitle: None,
                score: 0,
                plugin_id: "file_manager",
                metadata,
            });
        }

        let sidebar_items = get_sidebar_items();
        let mut drive_idx = 0;
        let mut drives_metadata = Vec::new();
        for (name, path, kind) in &sidebar_items {
            if *kind == "drive_root" || *kind == "drive_external" {
                drives_metadata.push((drive_idx, name.clone(), path.to_string_lossy().to_string(), kind.to_string()));
                drive_idx += 1;
            }
        }

        let context_menu_open = if let Ok(g) = self.context_menu_open.read() { *g } else { false };
        let context_menu_selected_idx = if let Ok(g) = self.context_menu_selected_idx.read() { *g } else { 0 };
        let input_dialog_open = if let Ok(g) = self.input_dialog_open.read() { *g } else { false };
        let input_dialog_title = if let Ok(g) = self.input_dialog_title.read() { g.clone() } else { String::new() };
        let input_dialog_buffer = if let Ok(g) = self.input_dialog_buffer.read() { g.clone() } else { String::new() };
        let properties_dialog_open = if let Ok(g) = self.properties_dialog_open.read() { *g } else { false };
        let properties_dialog_item_id = if let Ok(g) = self.properties_dialog_item_id.read() { g.clone() } else { String::new() };
        let delete_confirm_open = if let Ok(g) = self.delete_confirm_open.read() { *g } else { false };
        let delete_confirm_item_id = if let Ok(g) = self.delete_confirm_item_id.read() { g.clone() } else { String::new() };
        let delete_confirm_selected_idx = if let Ok(g) = self.delete_confirm_selected_idx.read() { *g } else { 0 };

        for item in &mut results {
            item.metadata.insert("input_dialog_open".to_string(), input_dialog_open.to_string());
            item.metadata.insert("input_dialog_title".to_string(), input_dialog_title.clone());
            item.metadata.insert("input_dialog_buffer".to_string(), input_dialog_buffer.clone());
            item.metadata.insert("properties_dialog_open".to_string(), properties_dialog_open.to_string());
            item.metadata.insert("properties_dialog_item_id".to_string(), properties_dialog_item_id.clone());
            item.metadata.insert("delete_confirm_open".to_string(), delete_confirm_open.to_string());
            item.metadata.insert("delete_confirm_item_id".to_string(), delete_confirm_item_id.clone());
            item.metadata.insert("delete_confirm_selected_idx".to_string(), delete_confirm_selected_idx.to_string());
            item.metadata.insert("current_dir".to_string(), current_path_str.clone());
            item.metadata.insert("sidebar_focused".to_string(), sidebar_focused.to_string());
            item.metadata.insert("focus_pane".to_string(), focus_pane.to_string());
            item.metadata.insert("sidebar_selected_idx".to_string(), sidebar_selected_idx.to_string());
            item.metadata.insert("total_dirs".to_string(), total_dirs.to_string());
            item.metadata.insert("total_files".to_string(), total_files.to_string());
            item.metadata.insert("fav_home_count".to_string(), home_count.to_string());
            item.metadata.insert("fav_desktop_count".to_string(), desktop_count.to_string());
            item.metadata.insert("fav_docs_count".to_string(), docs_count.to_string());
            item.metadata.insert("fav_downloads_count".to_string(), downloads_count.to_string());
            item.metadata.insert("fav_music_count".to_string(), music_count.to_string());
            item.metadata.insert("fav_pics_count".to_string(), pics_count.to_string());
            item.metadata.insert("fav_videos_count".to_string(), videos_count.to_string());
            item.metadata.insert("fav_trash_count".to_string(), trash_count.to_string());
            item.metadata.insert("disk_total".to_string(), total_size.clone());
            item.metadata.insert("disk_used".to_string(), used_size.clone());
            item.metadata.insert("disk_percent".to_string(), use_percent.clone());
            item.metadata.insert("clip_path".to_string(), clip_path_str.clone());
            item.metadata.insert("clip_op".to_string(), clip_op_str.clone());

            item.metadata.insert("fav_drives_count".to_string(), drive_idx.to_string());
            for (d_idx, d_name, d_path, d_kind) in &drives_metadata {
                item.metadata.insert(format!("fav_drive_{}_name", d_idx), d_name.clone());
                item.metadata.insert(format!("fav_drive_{}_path", d_idx), d_path.clone());
                item.metadata.insert(format!("fav_drive_{}_kind", d_idx), d_kind.clone());
            }

            item.metadata.insert("context_menu_open".to_string(), context_menu_open.to_string());
            item.metadata.insert("context_menu_selected_idx".to_string(), context_menu_selected_idx.to_string());
        }

        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let is_parent = item.metadata.get("is_parent").map(|s| s == "true").unwrap_or(false);
        let path_str = item.metadata.get("path")?;
        let real_path = Path::new(path_str);

        let mut preview = String::new();

        if is_parent {
            preview.push_str("# Parent Directory\n");
            preview.push_str(&format!("*Path: {}*\n\n", path_str));
            preview.push_str("Contents:\n");
            if let Ok(entries) = fs::read_dir(real_path) {
                let mut count = 0;
                for entry in entries.flatten() {
                    if count >= 10 {
                        preview.push_str("  ...\n");
                        break;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    let icon = if entry.path().is_dir() { "📁" } else { "📄" };
                    preview.push_str(&format!("  {} {}\n", icon, name));
                    count += 1;
                }
            }
        } else if real_path.is_dir() {
            preview.push_str(&format!("# Directory: {}\n", item.title));
            preview.push_str(&format!("*Path: {}*\n\n", path_str));
            
            let type_desc = get_file_type_desc(real_path, true);
            let size = self.get_dir_size_str(real_path);
            let permissions = get_permissions_str(real_path);
            let owner = get_owner_str(real_path);

            preview.push_str("| Metadata | Value |\n");
            preview.push_str("| :--- | :--- |\n");
            preview.push_str(&format!("| **Type** | {} |\n", type_desc));
            preview.push_str(&format!("| **Size** | {} |\n", size));
            preview.push_str(&format!("| **Permissions** | `{}` |\n", permissions));
            preview.push_str(&format!("| **Owner** | `{}` |\n\n", owner));

            preview.push_str("Contents:\n");
            if let Ok(entries) = fs::read_dir(real_path) {
                let mut count = 0;
                for entry in entries.flatten() {
                    if count >= 10 {
                        preview.push_str("  ...\n");
                        break;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    let icon = get_file_icon(&entry.path(), entry.path().is_dir());
                    preview.push_str(&format!("  {} {}\n", icon, name));
                    count += 1;
                }
            }
        } else {
            // It is a file
            preview.push_str(&format!("# File: {}\n", item.title));
            preview.push_str(&format!("*Path: {}*\n\n", path_str));

            let type_desc = get_file_type_desc(real_path, false);
            let size_bytes = fs::metadata(real_path).map(|m| m.len()).unwrap_or(0);
            let size = self.format_size(size_bytes);
            let permissions = get_permissions_str(real_path);
            let owner = get_owner_str(real_path);

            preview.push_str("| Metadata | Value |\n");
            preview.push_str("| :--- | :--- |\n");
            preview.push_str(&format!("| **Type** | {} |\n", type_desc));
            preview.push_str(&format!("| **Size** | {} |\n", size));
            preview.push_str(&format!("| **Permissions** | `{}` |\n", permissions));
            preview.push_str(&format!("| **Owner** | `{}` |\n\n", owner));

            // Content preview
            let ext = real_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "gif" || ext == "webp" {
                preview.push_str(&format!("[IMAGE: {}]\n", path_str));
            } else {
                match fs::read_to_string(real_path) {
                    Ok(content) => {
                        preview.push_str("Preview:\n```\n");
                        let lines: Vec<&str> = content.lines().take(10).collect();
                        preview.push_str(&lines.join("\n"));
                        if content.lines().count() > 10 {
                            preview.push_str("\n...");
                        }
                        preview.push_str("\n```\n");
                    }
                    Err(_) => {
                        preview.push_str("(Binary content)\n");
                    }
                }
            }
        }

        // Add visual cheatsheet
        preview.push_str("\n---\n### ⌨️ Dolphin Keys\n");
        preview.push_str("- **Left/Right**: Focus Sidebar / Files\n");
        preview.push_str("- **Enter**: Open File / Enter Folder\n");
        preview.push_str("- **Backspace**: Go up Folder\n");
        preview.push_str("- **Alt-c** / **Alt-x**: Copy / Cut\n");
        preview.push_str("- **Alt-v**: Paste item\n");
        preview.push_str("- **Alt-d**: Delete item\n");
        preview.push_str("- **Alt-r**: Rename to query\n");
        preview.push_str("- **Alt-n** / **Alt-f**: New Folder/File\n");
        preview.push_str("- **F4**: Open terminal\n");

        if let Some((clip_path, op)) = self.clipboard.read().ok().and_then(|g| g.clone()) {
            preview.push_str(&format!(
                "\n*Clipboard: {:?} [{:?}]*\n",
                op,
                clip_path.file_name().unwrap_or_default()
            ));
        }

        Some(preview)
    }

    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult {
        if item.id == "dummy_metadata_carrier" {
            return ExecutionResult::Success;
        }
        let is_dir = item.metadata.get("is_dir").map(|s| s == "true").unwrap_or(false);
        let path_str = match item.metadata.get("path") {
            Some(p) => p,
            None => return ExecutionResult::Message("Path not found".to_string()),
        };

        if is_dir {
            let target_path = PathBuf::from(path_str);
            if let Ok(mut mode) = self.view_mode.write() {
                *mode = ViewMode::Normal;
            }
            if let Ok(mut guard) = self.current_dir.write() {
                *guard = target_path;
            }
            ctx.new_query = Some(String::new());
            ctx.refresh_search = true;
            ExecutionResult::Success
        } else {
            let real_path = PathBuf::from(path_str);
            self.record_recent_file(real_path.clone());
            let path_str = real_path.to_string_lossy().to_string();

            let ext = real_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            let is_text = matches!(
                ext.as_str(),
                "txt" | "md" | "toml" | "json" | "yaml" | "yml" | "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "sh" | "html" | "css" | "go" | "java"
            );

            if is_text {
                ctx.run_command(ctx.editor.clone(), vec![path_str], true);
            } else {
                ctx.run_command("xdg-open".to_string(), vec![path_str], false);
            }
            ExecutionResult::Exit
        }
    }

    fn handle_key(
        &self,
        key: KeyEvent,
        query: &str,
        selected_item: Option<&SearchResult>,
        ctx: &mut Context,
    ) -> bool {
        let current_dir = match self.current_dir.read() {
            Ok(g) => g.clone(),
            Err(_) => return false,
        };

        // --- 0. Properties Dialog Active Input Interception ---
        let properties_dialog_open = if let Ok(g) = self.properties_dialog_open.read() { *g } else { false };
        if properties_dialog_open {
            if let Ok(mut open) = self.properties_dialog_open.write() {
                *open = false;
            }
            ctx.refresh_search = true;
            return true;
        }

        // --- 0. Delete Confirmation Active Input Interception ---
        let delete_confirm_open = if let Ok(g) = self.delete_confirm_open.read() { *g } else { false };
        if delete_confirm_open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    if let Ok(mut open) = self.delete_confirm_open.write() {
                        *open = false;
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Ok(mut open) = self.delete_confirm_open.write() {
                        *open = false;
                    }
                    let item_id = if let Ok(g) = self.delete_confirm_item_id.read() { g.clone() } else { String::new() };
                    if !item_id.is_empty() {
                        let path = PathBuf::from(&item_id);
                        let is_dir = path.is_dir();
                        let res = if is_dir {
                            fs::remove_dir_all(&path)
                        } else {
                            fs::remove_file(&path)
                        };
                        match res {
                            Ok(_) => {
                                let filename = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                                ctx.message = Some(format!("Deleted: {}", filename));
                            }
                            Err(e) => {
                                ctx.message = Some(format!("Delete failed: {e}"));
                            }
                        }
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Left => {
                    if let Ok(mut idx) = self.delete_confirm_selected_idx.write() {
                        *idx = 0; // Select "No"
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Right => {
                    if let Ok(mut idx) = self.delete_confirm_selected_idx.write() {
                        *idx = 1; // Select "Yes"
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Enter => {
                    let confirm_idx = if let Ok(g) = self.delete_confirm_selected_idx.read() { *g } else { 0 };
                    if let Ok(mut open) = self.delete_confirm_open.write() {
                        *open = false;
                    }
                    if confirm_idx == 1 {
                        let item_id = if let Ok(g) = self.delete_confirm_item_id.read() { g.clone() } else { String::new() };
                        if !item_id.is_empty() {
                            let path = PathBuf::from(&item_id);
                            let is_dir = path.is_dir();
                            let res = if is_dir {
                                fs::remove_dir_all(&path)
                            } else {
                                fs::remove_file(&path)
                            };
                            match res {
                                Ok(_) => {
                                    let filename = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                                    ctx.message = Some(format!("Deleted: {}", filename));
                                }
                                Err(e) => {
                                    ctx.message = Some(format!("Delete failed: {e}"));
                                }
                            }
                        }
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                _ => {
                    return true;
                }
            }
        }

        // --- 0. Input Dialog Active Input Interception ---
        let input_dialog_open = if let Ok(g) = self.input_dialog_open.read() { *g } else { false };
        if input_dialog_open {
            match key.code {
                KeyCode::Esc => {
                    if let Ok(mut open) = self.input_dialog_open.write() {
                        *open = false;
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Backspace => {
                    if let Ok(mut buf) = self.input_dialog_buffer.write() {
                        buf.pop();
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Char(c) => {
                    if let Ok(mut buf) = self.input_dialog_buffer.write() {
                        buf.push(c);
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Enter => {
                    let buf = if let Ok(g) = self.input_dialog_buffer.read() { g.clone() } else { String::new() };
                    let action = if let Ok(g) = self.input_dialog_action.read() { *g } else { InputDialogAction::NewFile };
                    
                    if let Ok(mut open) = self.input_dialog_open.write() {
                        *open = false;
                    }
                    ctx.refresh_search = true;

                    if !buf.is_empty() {
                        match action {
                            InputDialogAction::Rename => {
                                if let Some(selected) = selected_item {
                                    if selected.id != ".." {
                                        let selected_path = PathBuf::from(&selected.id);
                                        let target_path = current_dir.join(&buf);
                                        if target_path.exists() {
                                            ctx.message = Some("A file or folder with that name already exists".to_string());
                                        } else if let Err(e) = fs::rename(&selected_path, &target_path) {
                                            ctx.message = Some(format!("Rename failed: {e}"));
                                        } else {
                                            ctx.message = Some(format!("Renamed to: {}", buf));
                                            ctx.new_query = Some(String::new());
                                            ctx.focus_target = Some(buf.clone());
                                        }
                                    }
                                }
                            }
                            InputDialogAction::NewFile => {
                                let new_file = current_dir.join(&buf);
                                if new_file.exists() {
                                    ctx.message = Some("File already exists".to_string());
                                } else if let Err(e) = fs::write(&new_file, "") {
                                    ctx.message = Some(format!("Failed to create file: {e}"));
                                } else {
                                    ctx.message = Some(format!("Created file: {}", buf));
                                    ctx.new_query = Some(String::new());
                                    ctx.focus_target = Some(buf.clone());
                                }
                            }
                            InputDialogAction::NewFolder => {
                                let new_dir = current_dir.join(&buf);
                                if new_dir.exists() {
                                    ctx.message = Some("Folder already exists".to_string());
                                } else if let Err(e) = fs::create_dir_all(&new_dir) {
                                    ctx.message = Some(format!("Failed to create folder: {e}"));
                                } else {
                                    ctx.message = Some(format!("Created folder: {}", buf));
                                    ctx.new_query = Some(String::new());
                                    ctx.focus_target = Some(buf.clone());
                                }
                            }
                        }
                    }
                    return true;
                }
                _ => {
                    return true;
                }
            }
        }

        // --- 0. Context Menu Active Input Interception ---
        let menu_open = if let Ok(g) = self.context_menu_open.read() { *g } else { false };
        if menu_open {
            let options_count = 10;
            match key.code {
                KeyCode::Up => {
                    if let Ok(mut idx) = self.context_menu_selected_idx.write() {
                        if *idx > 0 {
                            *idx -= 1;
                        } else {
                            *idx = options_count - 1;
                        }
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Down => {
                    if let Ok(mut idx) = self.context_menu_selected_idx.write() {
                        if *idx < options_count - 1 {
                            *idx += 1;
                        } else {
                            *idx = 0;
                        }
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Esc | KeyCode::Char('m') | KeyCode::Char('M') => {
                    if let Ok(mut open) = self.context_menu_open.write() {
                        *open = false;
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Enter => {
                    // Close menu first
                    if let Ok(mut open) = self.context_menu_open.write() {
                        *open = false;
                    }
                    ctx.refresh_search = true;
                    
                    let menu_idx = self.context_menu_selected_idx.read().map(|i| *i).unwrap_or(0);
                    // Execute option
                    match menu_idx {
                        0 => { // Open
                            if let Some(selected) = selected_item {
                                let exec_res = self.execute(selected, ctx);
                                if let ExecutionResult::Exit = exec_res {
                                    ctx.exit_requested = true;
                                }
                            }
                        }
                        1 => { // Copy
                            if let Some(selected) = selected_item {
                                if selected.id != ".." {
                                    let selected_path = PathBuf::from(&selected.id);
                                    if let Ok(mut guard) = self.clipboard.write() {
                                        *guard = Some((selected_path, ClipboardOp::Copy));
                                    }
                                    ctx.message = Some(format!("Copied: {}", selected.title));
                                }
                            }
                        }
                        2 => { // Cut
                            if let Some(selected) = selected_item {
                                if selected.id != ".." {
                                    let selected_path = PathBuf::from(&selected.id);
                                    if let Ok(mut guard) = self.clipboard.write() {
                                        *guard = Some((selected_path, ClipboardOp::Cut));
                                    }
                                    ctx.message = Some(format!("Cut: {}", selected.title));
                                }
                            }
                        }
                        3 => { // Paste
                            let clip = self.clipboard.read().ok().and_then(|g| g.clone());
                            if let Some((src_path, op)) = clip {
                                let filename = src_path.file_name().unwrap_or_default();
                                let mut target_path = current_dir.join(filename);
                                if target_path.exists() {
                                    let stem = src_path.file_stem().and_then(|s| s.to_str()).unwrap_or("copy");
                                    let ext = src_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                                    let mut counter = 1;
                                    loop {
                                        let new_name = if ext.is_empty() {
                                            format!("{}_copy{}", stem, counter)
                                        } else {
                                            format!("{}_copy{}.{}", stem, counter, ext)
                                        };
                                        let test_path = current_dir.join(new_name);
                                        if !test_path.exists() {
                                            target_path = test_path;
                                            break;
                                        }
                                        counter += 1;
                                    }
                                }
                                let target_filename = target_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                if let Err(e) = paste_item(&src_path, &target_path, op) {
                                    ctx.message = Some(format!("Paste failed: {e}"));
                                } else {
                                    ctx.message = Some(format!("Pasted {}", target_filename));
                                    if let ClipboardOp::Cut = op {
                                        if let Ok(mut guard) = self.clipboard.write() {
                                            *guard = None;
                                        }
                                    }
                                }
                            } else {
                                ctx.message = Some("Clipboard is empty".to_string());
                            }
                        }
                        4 => { // Rename
                            if let Some(selected) = selected_item {
                                if selected.id != ".." {
                                    let current_name = Path::new(&selected.id)
                                        .file_name()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    if let Ok(mut open) = self.input_dialog_open.write() {
                                        *open = true;
                                    }
                                    if let Ok(mut title) = self.input_dialog_title.write() {
                                        *title = "Rename File/Folder".to_string();
                                    }
                                    if let Ok(mut buf) = self.input_dialog_buffer.write() {
                                        *buf = current_name;
                                    }
                                    if let Ok(mut action) = self.input_dialog_action.write() {
                                        *action = InputDialogAction::Rename;
                                    }
                                }
                            }
                        }
                        5 => { // Delete
                            if let Some(selected) = selected_item {
                                if selected.id != ".." {
                                    if let Ok(mut open) = self.delete_confirm_open.write() {
                                        *open = true;
                                    }
                                    if let Ok(mut id) = self.delete_confirm_item_id.write() {
                                        *id = selected.id.clone();
                                    }
                                    if let Ok(mut idx) = self.delete_confirm_selected_idx.write() {
                                        *idx = 0; // Default to "No"
                                    }
                                }
                            }
                        }
                        6 => { // New File
                            if let Ok(mut open) = self.input_dialog_open.write() {
                                *open = true;
                            }
                            if let Ok(mut title) = self.input_dialog_title.write() {
                                *title = "Create New File".to_string();
                            }
                            if let Ok(mut buf) = self.input_dialog_buffer.write() {
                                *buf = String::new();
                            }
                            if let Ok(mut action) = self.input_dialog_action.write() {
                                *action = InputDialogAction::NewFile;
                            }
                        }
                        7 => { // New Folder
                            if let Ok(mut open) = self.input_dialog_open.write() {
                                *open = true;
                            }
                            if let Ok(mut title) = self.input_dialog_title.write() {
                                *title = "Create New Folder".to_string();
                            }
                            if let Ok(mut buf) = self.input_dialog_buffer.write() {
                                *buf = String::new();
                            }
                            if let Ok(mut action) = self.input_dialog_action.write() {
                                *action = InputDialogAction::NewFolder;
                            }
                        }
                        8 => { // Properties
                            if let Some(selected) = selected_item {
                                if selected.id != ".." {
                                    if let Ok(mut open) = self.properties_dialog_open.write() {
                                        *open = true;
                                    }
                                    if let Ok(mut id) = self.properties_dialog_item_id.write() {
                                        *id = selected.id.clone();
                                    }
                                }
                            }
                        }
                        9 => {} // Cancel (already closed)
                        _ => {}
                    }
                    return true;
                }
                _ => {
                    return true;
                }
            }
        }

        let focus_pane = if let Ok(g) = self.focus_pane.read() { *g } else { 1 };

        // --- 1. Global Alt-Keys / F-Keys (Available in all panes) ---
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                // F4/Alt-T: Terminal
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    ctx.run_command(ctx.shell.clone(), vec![], true);
                    ctx.exit_requested = true;
                    let cd_cmd = format!("cd '{}' && exec {}", current_dir.to_string_lossy(), ctx.shell);
                    ctx.command_to_run = Some((ctx.shell.clone(), vec!["-c".to_string(), cd_cmd], true));
                    return true;
                }
                // Alt-O: Open in system graphical file manager
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    ctx.run_command("xdg-open".to_string(), vec![current_dir.to_string_lossy().to_string()], false);
                    ctx.message = Some("Opening folder in system file manager".to_string());
                    return true;
                }
                // Alt-H: Toggle showing hidden files
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    if let Ok(mut show) = self.show_hidden.write() {
                        *show = !*show;
                        ctx.message = Some(if *show {
                            "Show hidden files enabled".to_string()
                        } else {
                            "Show hidden files disabled".to_string()
                        });
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                // Alt-N: Create Folder
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    if let Ok(mut open) = self.input_dialog_open.write() {
                        *open = true;
                    }
                    if let Ok(mut title) = self.input_dialog_title.write() {
                        *title = "Create New Folder".to_string();
                    }
                    if let Ok(mut buf) = self.input_dialog_buffer.write() {
                        *buf = String::new();
                    }
                    if let Ok(mut action) = self.input_dialog_action.write() {
                        *action = InputDialogAction::NewFolder;
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                // Alt-F: Create File
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    if let Ok(mut open) = self.input_dialog_open.write() {
                        *open = true;
                    }
                    if let Ok(mut title) = self.input_dialog_title.write() {
                        *title = "Create New File".to_string();
                    }
                    if let Ok(mut buf) = self.input_dialog_buffer.write() {
                        *buf = String::new();
                    }
                    if let Ok(mut action) = self.input_dialog_action.write() {
                        *action = InputDialogAction::NewFile;
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                // Alt-V: Paste
                KeyCode::Char('v') | KeyCode::Char('V') => {
                    let clip = self.clipboard.read().ok().and_then(|g| g.clone());
                    if let Some((src_path, op)) = clip {
                        let filename = src_path.file_name().unwrap_or_default();
                        let mut target_path = current_dir.join(filename);
                        if target_path.exists() {
                            let stem = src_path.file_stem().and_then(|s| s.to_str()).unwrap_or("copy");
                            let ext = src_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                            let mut counter = 1;
                            loop {
                                let new_name = if ext.is_empty() {
                                    format!("{}_copy{}", stem, counter)
                                } else {
                                    format!("{}_copy{}.{}", stem, counter, ext)
                                };
                                let test_path = current_dir.join(new_name);
                                if !test_path.exists() {
                                    target_path = test_path;
                                    break;
                                }
                                counter += 1;
                            }
                        }
                        let target_filename = target_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if let Err(e) = paste_item(&src_path, &target_path, op) {
                            ctx.message = Some(format!("Paste failed: {e}"));
                        } else {
                            ctx.message = Some(format!("Pasted {}", target_filename));
                            if let ClipboardOp::Cut = op {
                                if let Ok(mut guard) = self.clipboard.write() {
                                    *guard = None;
                                }
                            }
                            ctx.refresh_search = true;
                        }
                    } else {
                        ctx.message = Some("Clipboard is empty".to_string());
                    }
                    return true;
                }
                _ => {}
            }
        }

        // Global F4 key (Terminal)
        if key.code == KeyCode::F(4) {
            ctx.run_command(ctx.shell.clone(), vec![], true);
            ctx.exit_requested = true;
            let cd_cmd = format!("cd '{}' && exec {}", current_dir.to_string_lossy(), ctx.shell);
            ctx.command_to_run = Some((ctx.shell.clone(), vec!["-c".to_string(), cd_cmd], true));
            return true;
        }

        // Global Backspace: Go to parent directory (if not typing in Path bar (2) or Search box (3))
        if key.code == KeyCode::Backspace && focus_pane != 2 && focus_pane != 3 {
            let mode = self.view_mode.read().map(|m| *m).unwrap_or(ViewMode::Normal);
            if mode != ViewMode::Normal {
                if let Ok(mut m) = self.view_mode.write() {
                    *m = ViewMode::Normal;
                }
                ctx.refresh_search = true;
                ctx.new_query = Some(String::new());
                return true;
            }

            if let Some(parent) = current_dir.parent() {
                let exited_dir_name = current_dir.file_name().map(|n| n.to_string_lossy().to_string());
                if let Ok(mut guard) = self.current_dir.write() {
                    *guard = parent.to_path_buf();
                }
                ctx.refresh_search = true;
                ctx.new_query = Some(String::new());
                ctx.focus_target = exited_dir_name;
                ctx.message = Some(format!("Navigated up to: {}", parent.display()));
            }
            return true;
        }

        // --- 2. Alt-Keys and Keys requiring selection (Available in all panes if an item is selected) ---
        if let Some(selected) = selected_item {
            if selected.id != ".." {
                let selected_path = PathBuf::from(&selected.id);
                if key.modifiers.contains(KeyModifiers::ALT) {
                    match key.code {
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            if let Ok(mut guard) = self.clipboard.write() {
                                *guard = Some((selected_path.clone(), ClipboardOp::Copy));
                            }
                            ctx.message = Some(format!("Copied: {}", selected.title));
                            return true;
                        }
                        KeyCode::Char('x') | KeyCode::Char('X') => {
                            if let Ok(mut guard) = self.clipboard.write() {
                                *guard = Some((selected_path.clone(), ClipboardOp::Cut));
                            }
                            ctx.message = Some(format!("Cut: {}", selected.title));
                            return true;
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            if let Ok(mut open) = self.delete_confirm_open.write() {
                                *open = true;
                            }
                            if let Ok(mut id) = self.delete_confirm_item_id.write() {
                                *id = selected.id.clone();
                            }
                            if let Ok(mut idx) = self.delete_confirm_selected_idx.write() {
                                *idx = 0;
                            }
                            ctx.refresh_search = true;
                            return true;
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            let current_name = selected_path.file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            if let Ok(mut open) = self.input_dialog_open.write() {
                                *open = true;
                            }
                            if let Ok(mut title) = self.input_dialog_title.write() {
                                *title = "Rename File/Folder".to_string();
                            }
                            if let Ok(mut buf) = self.input_dialog_buffer.write() {
                                *buf = current_name;
                            }
                            if let Ok(mut action) = self.input_dialog_action.write() {
                                *action = InputDialogAction::Rename;
                            }
                            ctx.refresh_search = true;
                            return true;
                        }
                        _ => {}
                    }
                } else if key.code == KeyCode::Delete {
                    if let Ok(mut open) = self.delete_confirm_open.write() {
                        *open = true;
                    }
                    if let Ok(mut id) = self.delete_confirm_item_id.write() {
                        *id = selected_path.to_string_lossy().to_string();
                    }
                    if let Ok(mut idx) = self.delete_confirm_selected_idx.write() {
                        *idx = 0; // Default to "No"
                    }
                    ctx.refresh_search = true;
                    return true;
                }
            }
        }

        // --- 3. Pane-switching Tab Keys ---
        if key.code == KeyCode::Tab || key.code == KeyCode::BackTab {
            let new_focus = if key.code == KeyCode::Tab {
                (focus_pane + 1) % 4
            } else {
                if focus_pane == 0 { 3 } else { focus_pane - 1 }
            };
            if let Ok(mut guard) = self.focus_pane.write() {
                *guard = new_focus;
            }
            if new_focus == 2 {
                // Focus path: fill query with current path string
                ctx.new_query = Some(current_dir.to_string_lossy().to_string());
            } else {
                // Return query to empty for searches
                ctx.new_query = Some(String::new());
            }
            
            let focus_name = match new_focus {
                0 => "Favorites sidebar",
                1 => "Files list",
                2 => "Path bar",
                3 => "Search box",
                _ => "",
            };
            ctx.message = Some(format!("Focused {}", focus_name));
            ctx.refresh_search = true;
            return true;
        }

        // --- 4. Focus-Specific Key Handlings (Non-Alt, non-global) ---

        // A. Favorites Sidebar Focus (focus_pane == 0)
        if focus_pane == 0 {
            match key.code {
                KeyCode::Right => {
                    if let Ok(mut guard) = self.focus_pane.write() {
                        *guard = 1; // Focus Files list
                    }
                    ctx.refresh_search = true;
                    ctx.message = Some("Focused Files list".to_string());
                    return true;
                }
                KeyCode::Up => {
                    let sidebar_items = get_sidebar_items();
                    let max_idx = sidebar_items.len().saturating_sub(1);
                    if let Ok(mut idx) = self.sidebar_selected_idx.write() {
                        if *idx > 0 {
                            *idx -= 1;
                        } else {
                            *idx = max_idx;
                        }
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Down => {
                    let sidebar_items = get_sidebar_items();
                    let max_idx = sidebar_items.len().saturating_sub(1);
                    if let Ok(mut idx) = self.sidebar_selected_idx.write() {
                        if *idx < max_idx {
                            *idx += 1;
                        } else {
                            *idx = 0;
                        }
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                KeyCode::Enter => {
                    let idx = self.sidebar_selected_idx.read().map(|i| *i).unwrap_or(0);
                    let sidebar_items = get_sidebar_items();
                    if idx < sidebar_items.len() {
                        let (_, target_path, kind) = &sidebar_items[idx];
                        if *kind == "recent_files" {
                            if let Ok(mut mode) = self.view_mode.write() {
                                *mode = ViewMode::RecentFiles;
                            }
                            if let Ok(mut guard) = self.focus_pane.write() {
                                *guard = 1;
                            }
                            ctx.new_query = Some(String::new());
                        } else if *kind == "recent_locations" {
                            if let Ok(mut mode) = self.view_mode.write() {
                                *mode = ViewMode::RecentDirs;
                            }
                            if let Ok(mut guard) = self.focus_pane.write() {
                                *guard = 1;
                            }
                            ctx.new_query = Some(String::new());
                        } else {
                            if let Ok(mut mode) = self.view_mode.write() {
                                *mode = ViewMode::Normal;
                            }
                            if let Ok(mut guard) = self.current_dir.write() {
                                *guard = target_path.clone();
                            }
                            if let Ok(mut guard) = self.focus_pane.write() {
                                *guard = 1; // Focus Files list
                            }
                            ctx.new_query = Some(String::new());
                        }
                    }
                    ctx.refresh_search = true;
                    return true;
                }
                _ => {}
            }
            if key.code != KeyCode::Esc {
                return true;
            }
            return false;
        }

        // B. Location Path Bar Focus (focus_pane == 2)
        if focus_pane == 2 {
            if key.code == KeyCode::Enter {
                let target_path = PathBuf::from(query.trim());
                if target_path.exists() && target_path.is_dir() {
                    if let Ok(mut mode) = self.view_mode.write() {
                        *mode = ViewMode::Normal;
                    }
                    if let Ok(mut guard) = self.current_dir.write() {
                        *guard = target_path.clone();
                    }
                    if let Ok(mut guard) = self.focus_pane.write() {
                        *guard = 1; // Jump back to files list
                    }
                    ctx.new_query = Some(String::new());
                    ctx.refresh_search = true;
                    ctx.message = Some(format!("Navigated to: {}", target_path.display()));
                } else {
                    ctx.message = Some("Invalid or unreachable directory path".to_string());
                }
                return true;
            }
            return false;
        }

        // C. Search Box Focus (focus_pane == 3)
        if focus_pane == 3 {
            if key.code == KeyCode::Enter {
                if let Ok(mut guard) = self.focus_pane.write() {
                    *guard = 1; // Focus Files list
                }
                ctx.refresh_search = true;
                return true;
            }
            return false;
        }

        // D. Files List Focus (focus_pane == 1)
        if focus_pane == 1 {
            match key.code {
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    if selected_item.is_some() {
                        if let Ok(mut open) = self.context_menu_open.write() {
                            *open = true;
                        }
                        if let Ok(mut idx) = self.context_menu_selected_idx.write() {
                            *idx = 0;
                        }
                        ctx.refresh_search = true;
                        return true;
                    }
                }
                KeyCode::Left => {
                    if let Ok(mut guard) = self.focus_pane.write() {
                        *guard = 0; // Focus Sidebar
                    }
                    ctx.refresh_search = true;
                    ctx.message = Some("Focused Favorites sidebar".to_string());
                    return true;
                }
                _ => {}
            }
        }

        // Block normal character input in list view to avoid leaks into query
        if let KeyCode::Char(_) = key.code {
            if !key.modifiers.contains(KeyModifiers::ALT) && !key.modifiers.contains(KeyModifiers::CONTROL) {
                return true;
            }
        }

        false
    }
}
