use super::model::{ContentHash, FileId, FileVersionId, SnapshotId, WorkspaceId};
use rusqlite::{params, Connection, Result, Transaction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileIdentityResolution {
    Existing(FileId),
    New(FileId),
    Ambiguous(Vec<FileId>),
}

#[derive(Debug, Clone)]
pub struct ObservedFile {
    pub path: String,
    pub bytes: Vec<u8>,
    pub hash: ContentHash,
}

impl ObservedFile {
    pub fn new(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        let hash = ContentHash::compute(&bytes);
        Self {
            path: path.into(),
            bytes,
            hash,
        }
    }
}

pub struct FileIdentityService<'a> {
    conn: &'a Connection,
}

impl<'a> FileIdentityService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// کشف و تصمیم‌گیری قطعی روی هویت فایل بر اساس سوابق تاریخی اسنپ‌شات قبلی
    pub fn resolve_identity(
        &self,
        _workspace_id: &WorkspaceId,
        parent_snapshot_id: Option<&SnapshotId>,
        path: &str,
        hash: &ContentHash,
    ) -> Result<FileIdentityResolution> {
        // ۱. بررسی تطابق مستقیم مسیر در اسنپ‌شات والد
        if let Some(parent_snap) = parent_snapshot_id {
            let direct_match: Option<String> = self
                .conn
                .query_row(
                    "SELECT file_id FROM snapshot_files WHERE snapshot_id = ?1 AND path_at_snapshot = ?2",
                    params![parent_snap.as_str(), path],
                    |row| row.get(0),
                )
                .ok();

            if let Some(fid) = direct_match {
                return Ok(FileIdentityResolution::Existing(FileId(fid)));
            }
        }

        // ۲. شناسایی سناریوی Move/Rename: بررسی فایل‌های ناپدیدشده از والد که هش یکسان دارند
        if let Some(parent_snap) = parent_snapshot_id {
            let mut stmt = self.conn.prepare(
                "SELECT sf.file_id, sf.path_at_snapshot FROM snapshot_files sf
                 JOIN file_versions fv ON sf.file_version_id = fv.file_version_id
                 WHERE sf.snapshot_id = ?1 AND fv.content_hash = ?2",
            )?;

            let candidates: Vec<String> = stmt
                .query_map(params![parent_snap.as_str(), hash.as_str()], |row| {
                    row.get(0)
                })?
                .filter_map(|r| r.ok())
                .collect();

            match candidates.len() {
                0 => {}
                1 => {
                    return Ok(FileIdentityResolution::Existing(FileId(
                        candidates[0].clone(),
                    )))
                }
                _ => {
                    return Ok(FileIdentityResolution::Ambiguous(
                        candidates.into_iter().map(FileId).collect(),
                    ));
                }
            }
        }

        // ۳. در غیر این صورت، موجودیت فایل کاملاً جدید است و شناسه Opaque تخصیص می‌یابد
        Ok(FileIdentityResolution::New(FileId::generate()))
    }

    /// ایجاد نسخه جدید فایل در صورتی که محتوای آن نسبت به رکورد قبلی فرق کرده باشد
    pub fn get_or_create_version(
        tx: &Transaction,
        file_id: &FileId,
        hash: &ContentHash,
        byte_size: usize,
        now: i64,
    ) -> Result<FileVersionId> {
        let existing: Option<String> = tx
            .query_row(
                "SELECT file_version_id FROM file_versions WHERE file_id = ?1 AND content_hash = ?2",
                params![file_id.as_str(), hash.as_str()],
                |row| row.get(0),
            )
            .ok();

        if let Some(ver_id) = existing {
            return Ok(FileVersionId(ver_id));
        }

        let new_ver = FileVersionId::generate();
        tx.execute(
            "INSERT INTO file_versions (file_version_id, file_id, content_hash, byte_size, created_at, parent_file_version_id)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![
                new_ver.as_str(),
                file_id.as_str(),
                hash.as_str(),
                byte_size as i64,
                now
            ],
        )?;

        Ok(new_ver)
    }
}

pub struct SnapshotBuilder;

impl SnapshotBuilder {
    /// ساخت و کامیت یک اسنپ‌شات اتمیک و تغییرناپذیر از مجموعه فایل‌های مشاهده‌شده
    pub fn commit_snapshot(
        conn: &mut Connection,
        workspace_id: &WorkspaceId,
        observed_files: &[ObservedFile],
    ) -> Result<SnapshotId> {
        let parent_snapshot_id: Option<SnapshotId> = conn
            .query_row(
                "SELECT current_snapshot_id FROM workspaces WHERE workspace_id = ?1",
                [workspace_id.as_str()],
                |row| row.get(0),
            )
            .ok()
            .flatten()
            .map(SnapshotId);

        let identity_service = FileIdentityService::new(conn);
        let mut bindings = Vec::new();

        for obs in observed_files {
            let resolution = identity_service.resolve_identity(
                workspace_id,
                parent_snapshot_id.as_ref(),
                &obs.path,
                &obs.hash,
            )?;

            let file_id = match resolution {
                FileIdentityResolution::Existing(fid) => fid,
                FileIdentityResolution::New(fid) => fid,
                FileIdentityResolution::Ambiguous(_candidates) => FileId::generate(),
            };

            bindings.push((file_id, obs));
        }

        // ترنزکشن اتمیک All-or-Nothing در SQLite
        let tx = conn.transaction()?;
        let snapshot_id = SnapshotId::generate();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        tx.execute(
            "INSERT INTO snapshots (snapshot_id, workspace_id, parent_snapshot_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                snapshot_id.as_str(),
                workspace_id.as_str(),
                parent_snapshot_id.as_ref().map(|p| p.as_str()),
                now
            ],
        )?;

        for (file_id, obs) in bindings {
            tx.execute(
                "INSERT OR IGNORE INTO files (file_id, workspace_id, created_at, deleted_at)
                 VALUES (?1, ?2, ?3, NULL)",
                params![file_id.as_str(), workspace_id.as_str(), now],
            )?;

            let version_id = FileIdentityService::get_or_create_version(
                &tx,
                &file_id,
                &obs.hash,
                obs.bytes.len(),
                now,
            )?;

            tx.execute(
                "INSERT INTO snapshot_files (snapshot_id, file_id, path_at_snapshot, file_version_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    snapshot_id.as_str(),
                    file_id.as_str(),
                    obs.path,
                    version_id.as_str()
                ],
            )?;
        }

        tx.execute(
            "UPDATE workspaces SET current_snapshot_id = ?1 WHERE workspace_id = ?2",
            params![snapshot_id.as_str(), workspace_id.as_str()],
        )?;

        tx.commit()?;
        Ok(snapshot_id)
    }
}
