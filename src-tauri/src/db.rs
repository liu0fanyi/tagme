use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Manager;
use std::path::Path;
use sha2::{Sha256, Digest};
use std::fs;
use std::time::SystemTime;

// Lightweight file listing for scan (no hash, not in DB yet)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileListItem {
    pub path: String,
    pub size_bytes: u64,
    pub last_modified: i64,
    pub is_directory: bool,
}

// Full file info for files in database (with hash)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub id: u32,
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub last_modified: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagInfo {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
    pub color: Option<String>,
    pub position: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub pinned: bool,
}

fn get_db_path(app_handle: &AppHandle) -> std::path::PathBuf {
    app_handle
        .path()
        .app_data_dir()
        .expect("failed to get app data dir")
        .join("tagme_app.db")
}

pub fn init_db(app_handle: &AppHandle) -> Result<()> {
    let db_path = get_db_path(app_handle);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create app data dir");
    }

    let conn = Connection::open(&db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            content_hash TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            last_modified INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            parent_id INTEGER,
            color TEXT,
            position INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (parent_id) REFERENCES tags(id) ON DELETE CASCADE,
            UNIQUE(name, parent_id)
        )",
        [],
    )?;

    // Migration: Add position column if it doesn't exist
    let _ = conn.execute(
        "ALTER TABLE tags ADD COLUMN position INTEGER NOT NULL DEFAULT 0",
        [],
    );

    // Initialize positions for existing tags (group by parent_id)
    conn.execute(
        "UPDATE tags SET position = (
            SELECT COUNT(*) FROM tags t2 
            WHERE (t2.parent_id IS tags.parent_id OR (t2.parent_id IS NULL AND tags.parent_id IS NULL))
            AND t2.id < tags.id
        ) WHERE position = 0",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS file_tags (
            file_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (file_id, tag_id),
            FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS window_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            width REAL NOT NULL,
            height REAL NOT NULL,
            x REAL NOT NULL,
            y REAL NOT NULL,
            pinned INTEGER NOT NULL
        )",
        [],
    )?;

    // 检查是否有任何tag数据，如果没有则创建默认tag
    let tag_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tags",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    if tag_count == 0 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        eprintln!("🏷️  数据库为空，正在创建默认tag...");

        // 创建默认的tag
        let default_tags = vec![
            ("工作", None, Some("#FF6B6B")),
            ("个人", None, Some("#4ECDC4")),
            ("重要", None, Some("#45B7D1")),
            ("项目A", Some(1), Some("#96CEB4")),
            ("项目B", Some(1), Some("#FECA57")),
            ("学习", Some(2), Some("#DDA0DD")),
            ("娱乐", Some(2), Some("#98D8C8")),
        ];

        for (name, parent_id, color) in default_tags {
            conn.execute(
                "INSERT INTO tags (name, parent_id, color, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![name, parent_id, color, now],
            )?;
            eprintln!("   ✅ 创建tag: {}", name);
        }

        eprintln!("🎉 默认tag创建完成！");
    }

    Ok(())
}

// Settings functions
pub fn set_root_directory(app_handle: &AppHandle, path: String) -> Result<()> {
    let conn = Connection::open(get_db_path(app_handle))?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('root_directory', ?1)",
        params![path],
    )?;
    Ok(())
}

pub fn get_root_directory(app_handle: &AppHandle) -> Result<Option<String>> {
    let conn = Connection::open(get_db_path(app_handle))?;
    let result = conn.query_row(
        "SELECT value FROM settings WHERE key = 'root_directory'",
        [],
        |row| row.get(0),
    );
    match result {
        Ok(path) => Ok(Some(path)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn set_root_directories(app_handle: &AppHandle, paths: Vec<String>) -> Result<()> {
    let conn = Connection::open(get_db_path(app_handle))?;
    let value = serde_json::to_string(&paths).unwrap_or("[]".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('root_directories', ?1)",
        params![value],
    )?;
    Ok(())
}

pub fn get_root_directories(app_handle: &AppHandle) -> Result<Vec<String>> {
    let conn = Connection::open(get_db_path(app_handle))?;
    let result: Result<String, _> = conn.query_row(
        "SELECT value FROM settings WHERE key = 'root_directories'",
        [],
        |row| row.get(0),
    );
    match result {
        Ok(json) => serde_json::from_str::<Vec<String>>(&json)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

pub fn add_root_directory(app_handle: &AppHandle, path: String) -> Result<()> {
    let mut list = get_root_directories(app_handle)?;
    if !list.iter().any(|p| p == &path) {
        list.push(path);
    }
    set_root_directories(app_handle, list)
}

pub fn remove_root_directory(app_handle: &AppHandle, path: String) -> Result<()> {
    let mut list = get_root_directories(app_handle)?;
    list.retain(|p| p != &path);
    set_root_directories(app_handle, list)
}

// File hashing function
fn hash_file_content(path: &Path) -> Result<String, std::io::Error> {
    let file = fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    std::io::copy(&mut reader, &mut hasher)?;
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

// Lightweight file scanning - just list files, no hashing or DB operations
pub fn scan_directory_lightweight(root_path: String) -> Result<Vec<FileListItem>, std::io::Error> {
    eprintln!("🔍 Starting lightweight scan for directory: {}", root_path);
    
    let mut scanned_items = Vec::new();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Non-recursive scan: read both files and directories in the directory
    println!("📂 Reading directory entries...");
    for entry in fs::read_dir(&root_path)? {
        if let Ok(entry) = entry {
            if let Ok(file_type) = entry.file_type() {
                let path = entry.path();
                let path_str = path.to_string_lossy().to_string();
                
                if file_type.is_file() {
                    // Regular file
                    if let Ok(metadata) = fs::metadata(&path) {
                        let size_bytes = metadata.len();
                        let last_modified = metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(now);

                        scanned_items.push(FileListItem {
                            path: path_str,
                            size_bytes,
                            last_modified,
                            is_directory: false,
                        });
                    }
                } else if file_type.is_dir() {
                    // Directory - include it but don't recurse
                    if let Ok(metadata) = fs::metadata(&path) {
                        let last_modified = metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(now);

                        scanned_items.push(FileListItem {
                            path: path_str,
                            size_bytes: 0, // Directories have no size
                            last_modified,
                            is_directory: true,
                        });
                    }
                }
            }
        }
    }

    eprintln!("✅ Lightweight scan complete! Found {} items ({} files + {} folders)", 
        scanned_items.len(),
        scanned_items.iter().filter(|i| !i.is_directory).count(),
        scanned_items.iter().filter(|i| i.is_directory).count()
    );
    Ok(scanned_items)
}

pub fn scan_directories_lightweight(root_paths: Vec<String>) -> Result<Vec<FileListItem>, std::io::Error> {
    let mut all = Vec::new();
    for root in root_paths {
        let mut items = scan_directory_lightweight(root)?;
        all.append(&mut items);
    }
    Ok(all)
}

// Prune files from DB that no longer exist on disk
pub fn prune_missing_files(app_handle: &AppHandle) -> Result<()> {
    let conn = Connection::open(get_db_path(app_handle))?;
    
    // Get all files from DB
    let mut stmt = conn.prepare("SELECT id, path FROM files")?;
    let files_iter = stmt.query_map([], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut ids_to_delete = Vec::new();

    for file_result in files_iter {
        if let Ok((id, path)) = file_result {
            if !Path::new(&path).exists() {
                eprintln!("🗑️ File not found on disk, marking for deletion: {}", path);
                ids_to_delete.push(id);
            }
        }
    }

    if !ids_to_delete.is_empty() {
        eprintln!("🗑️ Pruning {} missing files from database...", ids_to_delete.len());
        // Delete in batches or one by one
        for id in ids_to_delete {
            conn.execute("DELETE FROM files WHERE id = ?1", params![id])?;
        }
        eprintln!("✅ Pruning complete");
    } else {
        eprintln!("✨ No missing files found in database");
    }

    Ok(())
}

// Hash and insert file into database (called when tagging a file)
// Returns file_id of existing or newly inserted file
pub fn hash_and_insert_file(app_handle: &AppHandle, path: String) -> Result<u32> {
    let conn = Connection::open(get_db_path(app_handle))?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let path_obj = Path::new(&path);
    
    // Get file metadata
    let metadata = fs::metadata(&path_obj)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let size_bytes = metadata.len();
    let last_modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(now);

    // Check if file exists in DB
    let existing: Option<(u32, String, i64, i64)> = conn
        .query_row(
            "SELECT id, content_hash, size_bytes, last_modified FROM files WHERE path = ?1",
            params![path],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok();

    let file_id = if let Some((id, _old_hash, old_size, old_mtime)) = existing {
        eprintln!("📄 File exists in DB (id: {})", id);
        
        // Early cutoff: if size and mtime match, reuse old hash
        if old_size == size_bytes as i64 && old_mtime == last_modified {
            eprintln!("   └─ ✨ Metadata unchanged - reusing cached hash");
            id
        } else {
            // Metadata changed, need to re-hash
            eprintln!("   └─ Metadata changed, re-hashing...");
            let new_hash = hash_file_content(&path_obj)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            
            conn.execute(
                "UPDATE files SET content_hash = ?1, size_bytes = ?2, last_modified = ?3, updated_at = ?4 WHERE id = ?5",
                params![new_hash, size_bytes as i64, last_modified, now, id],
            )?;
            eprintln!("   └─ ✅ Updated in DB");
            id
        }
    } else {
        // New file - must hash and insert
        eprintln!("📄 New file, hashing and inserting: {}", path);
        let content_hash = hash_file_content(&path_obj)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        
        conn.execute(
            "INSERT INTO files (path, content_hash, size_bytes, last_modified, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![path, content_hash, size_bytes as i64, last_modified, now, now],
        )?;
        let new_id = conn.last_insert_rowid() as u32;
        eprintln!("   └─ ✅ Inserted with id: {}", new_id);
        new_id
    };

    Ok(file_id)
}


// Get all files
pub fn get_all_files(app_handle: &AppHandle) -> Result<Vec<FileInfo>> {
    let conn = Connection::open(get_db_path(app_handle))?;
    let mut stmt = conn.prepare(
        "SELECT id, path, content_hash, size_bytes, last_modified FROM files ORDER BY path",
    )?;

    let files = stmt
        .query_map([], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                content_hash: row.get(2)?,
                size_bytes: row.get::<_, i64>(3)? as u64,
                last_modified: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(files)
}

// Tag CRUD operations
pub fn create_tag(
    app_handle: &AppHandle,
    name: String,
    parent_id: Option<u32>,
    color: Option<String>,
) -> Result<u32> {
    let conn = Connection::open(get_db_path(app_handle))?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Get max position for this parent
    let max_position: i32 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) FROM tags WHERE parent_id IS ?1",
        params![parent_id],
        |row| row.get(0),
    ).unwrap_or(-1);
    
    let new_position = max_position + 1;

    conn.execute(
        "INSERT INTO tags (name, parent_id, color, position, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, parent_id, color, new_position, now],
    )?;

    Ok(conn.last_insert_rowid() as u32)
}

pub fn get_all_tags(app_handle: &AppHandle) -> Result<Vec<TagInfo>> {
    eprintln!("🏷️  [DB] get_all_tags called");
    let conn = Connection::open(get_db_path(app_handle))?;
    let mut stmt = conn.prepare("SELECT id, name, parent_id, color, position FROM tags ORDER BY parent_id, position")?;

    let tags = stmt
        .query_map([], |row| {
            Ok(TagInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                color: row.get(3)?,
                position: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    eprintln!("🏷️  [DB] Found {} tags", tags.len());
    for tag in &tags {
        eprintln!("   - DB: Tag: {}, ID: {}, Parent: {:?}, Pos: {}",
            tag.name, tag.id, tag.parent_id, tag.position);
    }
    Ok(tags)
}

pub fn update_tag(
    app_handle: &AppHandle,
    id: u32,
    name: String,
    color: Option<String>,
) -> Result<()> {
    let conn = Connection::open(get_db_path(app_handle))?;
    conn.execute(
        "UPDATE tags SET name = ?1, color = ?2 WHERE id = ?3",
        params![name, color, id],
    )?;
    Ok(())
}

pub fn delete_tag(app_handle: &AppHandle, id: u32) -> Result<()> {
    let conn = Connection::open(get_db_path(app_handle))?;
    conn.execute("DELETE FROM tags WHERE id = ?1", params![id])?;
    Ok(())
}

// Helper function to reorder tags after a move
fn reorder_tags_in_parent(conn: &Connection, parent_id: Option<u32>) -> Result<()> {
    eprintln!("🔧 [DB] reorder_tags_in_parent: parent={:?}", parent_id);
    // Get all tags in this parent, ordered by current position
    let mut stmt = conn.prepare(
        "SELECT id FROM tags WHERE parent_id IS ?1 ORDER BY position"
    )?;

    let tag_ids: Vec<u32> = stmt
        .query_map(params![parent_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    eprintln!("🔧 [DB] Found {} tags to reorder: {:?}", tag_ids.len(), tag_ids);

    // Reassign positions sequentially
    for (index, tag_id) in tag_ids.iter().enumerate() {
        conn.execute(
            "UPDATE tags SET position = ?1 WHERE id = ?2",
            params![index as i32, tag_id],
        )?;
    }

    eprintln!("🔧 [DB] Reorder completed for parent {:?}", parent_id);
    Ok(())
}

pub fn move_tag(
    app_handle: &AppHandle,
    id: u32,
    new_parent_id: Option<u32>,
    target_position: i32,
) -> Result<()> {
    eprintln!("🔄 [DB] move_tag called: id={}, new_parent={:?}, target_pos={}", id, new_parent_id, target_position);
    let conn = Connection::open(get_db_path(app_handle))?;

    // Get current parent
    let old_parent_id: Option<u32> = conn.query_row(
        "SELECT parent_id FROM tags WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;

    eprintln!("🔄 [DB] Current parent of tag {}: {:?}", id, old_parent_id);

    // If moving within the same parent, shift positions of affected tags
    if old_parent_id == new_parent_id {
        eprintln!("🔄 [DB] Moving within same parent, shifting positions");
        let current_pos: i32 = conn.query_row(
            "SELECT position FROM tags WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        if current_pos < target_position {
            // Moving forward: shift tags between current_pos+1 and target_position down by 1
            conn.execute(
                "UPDATE tags SET position = position - 1 WHERE parent_id IS ?1 AND position > ?2 AND position <= ?3 AND id != ?4",
                params![new_parent_id, current_pos, target_position, id],
            )?;
        } else if current_pos > target_position {
            // Moving backward: shift tags between target_position and current_pos-1 up by 1
            conn.execute(
                "UPDATE tags SET position = position + 1 WHERE parent_id IS ?1 AND position >= ?2 AND position < ?3 AND id != ?4",
                params![new_parent_id, target_position, current_pos, id],
            )?;
        }
    }

    // Update parent and position
    conn.execute(
        "UPDATE tags SET parent_id = ?1, position = ?2 WHERE id = ?3",
        params![new_parent_id, target_position, id],
    )?;

    eprintln!("🔄 [DB] Updated tag {} to parent {:?}, position {}", id, new_parent_id, target_position);

    // Reorder tags in both old and new parents (only if different parents)
    if old_parent_id != new_parent_id {
        eprintln!("🔄 [DB] Reordering old parent {:?}", old_parent_id);
        reorder_tags_in_parent(&conn, old_parent_id)?;
        eprintln!("🔄 [DB] Reordering new parent {:?}", new_parent_id);
        reorder_tags_in_parent(&conn, new_parent_id)?;
    }

    eprintln!("🔄 [DB] move_tag completed successfully");
    Ok(())
}

// File-tag relationship operations
// Now accepts file_path instead of file_id - will hash and insert file if needed
pub fn add_file_tag(app_handle: &AppHandle, file_path: String, tag_id: u32) -> Result<()> {
    // First, ensure file is in database (hash if needed)
    let file_id = hash_and_insert_file(app_handle, file_path)?;
    
    // Now add the tag relationship
    let conn = Connection::open(get_db_path(app_handle))?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "INSERT OR IGNORE INTO file_tags (file_id, tag_id, created_at) VALUES (?1, ?2, ?3)",
        params![file_id, tag_id, now],
    )?;
    
    eprintln!("✅ Tag {} added to file {}", tag_id, file_id);
    Ok(())
}

pub fn remove_file_tag(app_handle: &AppHandle, file_id: u32, tag_id: u32) -> Result<()> {
    let conn = Connection::open(get_db_path(app_handle))?;
    conn.execute(
        "DELETE FROM file_tags WHERE file_id = ?1 AND tag_id = ?2",
        params![file_id, tag_id],
    )?;
    Ok(())
}

pub fn get_file_tags(app_handle: &AppHandle, file_id: u32) -> Result<Vec<TagInfo>> {
    let conn = Connection::open(get_db_path(app_handle))?;
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.parent_id, t.color, t.position
         FROM tags t
         JOIN file_tags ft ON t.id = ft.tag_id
         WHERE ft.file_id = ?1
         ORDER BY t.name",
    )?;

    let tags = stmt
        .query_map(params![file_id], |row| {
            Ok(TagInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                color: row.get(3)?,
                position: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(tags)
}

pub fn get_files_by_tags(
    app_handle: &AppHandle,
    tag_ids: Vec<u32>,
    use_and_logic: bool,
) -> Result<Vec<FileInfo>> {
    let conn = Connection::open(get_db_path(app_handle))?;

    if tag_ids.is_empty() {
        return get_all_files(app_handle);
    }

    let query = if use_and_logic {
        // AND logic: files must have ALL selected tags
        format!(
            "SELECT DISTINCT f.id, f.path, f.content_hash, f.size_bytes, f.last_modified
             FROM files f
             WHERE (SELECT COUNT(DISTINCT ft.tag_id) 
                    FROM file_tags ft 
                    WHERE ft.file_id = f.id AND ft.tag_id IN ({})) = {}
             ORDER BY f.path",
            tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(","),
            tag_ids.len()
        )
    } else {
        // OR logic: files must have ANY selected tag
        format!(
            "SELECT DISTINCT f.id, f.path, f.content_hash, f.size_bytes, f.last_modified
             FROM files f
             JOIN file_tags ft ON f.id = ft.file_id
             WHERE ft.tag_id IN ({})
             ORDER BY f.path",
            tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",")
        )
    };

    let mut stmt = conn.prepare(&query)?;
    let params: Vec<_> = tag_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

    let files = stmt
        .query_map(&params[..], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                content_hash: row.get(2)?,
                size_bytes: row.get::<_, i64>(3)? as u64,
                last_modified: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(files)
}

// Window state management
pub fn save_window_state(
    app_handle: &AppHandle,
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    pinned: bool,
) -> Result<()> {
    let conn = Connection::open(get_db_path(app_handle))?;
    conn.execute(
        "INSERT OR REPLACE INTO window_state (id, width, height, x, y, pinned)
         VALUES (1, ?1, ?2, ?3, ?4, ?5)",
        params![width, height, x, y, pinned as i32],
    )?;
    Ok(())
}

pub fn load_window_state(app_handle: &AppHandle) -> Result<Option<WindowState>> {
    let conn = Connection::open(get_db_path(app_handle))?;
    let result = conn.query_row(
        "SELECT width, height, x, y, pinned FROM window_state WHERE id = 1",
        [],
        |row| {
            Ok(WindowState {
                width: row.get(0)?,
                height: row.get(1)?,
                x: row.get(2)?,
                y: row.get(3)?,
                pinned: row.get::<_, i32>(4)? != 0,
            })
        },
    );

    match result {
        Ok(state) => Ok(Some(state)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}
