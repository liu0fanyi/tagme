use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileListItem {
    pub path: String,
    pub size_bytes: u64,
    pub last_modified: i64,
    pub is_directory: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: u32,
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub last_modified: i64,
    pub is_directory: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TagInfo {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
    pub color: Option<String>,
    pub position: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileWithTags {
    pub file: FileInfo,
    pub tags: Vec<TagInfo>,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum SortColumn {
    Name,
    Size,
    Date,
    Type,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayFile {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub last_modified: i64,
    pub db_id: Option<u32>,
    pub tags: Vec<TagInfo>,
    pub is_directory: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagArgs {
    pub name: String,
    pub parent_id: Option<u32>,
    pub color: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagArgs {
    pub id: u32,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteTagArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveTagArgs {
    pub id: u32,
    pub new_parent_id: Option<u32>,
    pub target_position: i32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddFileTagArgs {
    pub file_path: String,
    pub tag_id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveFileTagArgs {
    pub file_id: u32,
    pub tag_id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFileTagsArgs {
    pub file_id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterFilesByTagsArgs {
    pub tag_ids: Vec<u32>,
    pub use_and_logic: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFilesArgs {
    pub root_path: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFileArgs {
    pub path: String,
}
