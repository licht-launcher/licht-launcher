/// One resolved CurseForge file, as stored in the `curseforge_files` table.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub(crate) struct StoredCurseforgeFile {
    pub relative_path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub size: i64,
    pub project_id: i64,
    pub file_id: i64,
    pub file_name: String,
    pub name: String,
    pub required: bool,
}
