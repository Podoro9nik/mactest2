use super::types::{AppResult, Consent, Job, Record, RemoteTask, WorkState};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub(crate) struct Store {
    pub root: PathBuf,
    records: Mutex<HashMap<String, Record>>,
    _instance_lock: File,
}

impl Store {
    pub fn open(root: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(root.join("jobs")).map_err(|_| "save_failed")?;
        std::fs::create_dir_all(root.join("outputs")).map_err(|_| "save_failed")?;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("rune-instance.lock"))
            .map_err(|_| "save_failed")?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| "already_running")?;
        let mut files: Vec<_> = std::fs::read_dir(root.join("jobs"))
            .map_err(|_| "save_failed")?
            .flatten()
            .map(|entry| entry.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        // New r_ records override the corresponding read-only c_ legacy import.
        files.sort();
        let mut records = HashMap::new();
        for path in files.into_iter().take(5000) {
            let Ok(file) = File::open(path) else { continue };
            let mut bytes = Vec::new();
            if file.take(1024 * 1024).read_to_end(&mut bytes).is_err() {
                continue;
            }
            let parsed = serde_json::from_slice::<Record>(&bytes)
                .ok()
                .filter(|r| r.schema == 4 && valid_id(&r.id))
                .or_else(|| legacy(&bytes));
            let Some(mut record) = parsed else { continue };
            match &record.state {
                WorkState::Preparing => {
                    record.state = WorkState::Failed {
                        code: "interrupted".into(),
                        remote: None,
                    }
                }
                WorkState::Submitting { .. } => record.last_error = Some("submit_uncertain".into()),
                _ => (),
            }
            records.insert(record.id.clone(), record);
        }
        Ok(Self {
            root,
            records: Mutex::new(records),
            _instance_lock: lock,
        })
    }

    pub fn read(&self, id: &str) -> AppResult<Record> {
        if !valid_id(id) {
            return Err("invalid_job_id".into());
        }
        self.records
            .lock()
            .map_err(|_| "save_failed")?
            .get(id)
            .cloned()
            .ok_or_else(|| "job_not_found".into())
    }

    pub fn find_remote(&self, id: &str) -> AppResult<Option<Record>> {
        Ok(self
            .records
            .lock()
            .map_err(|_| "save_failed")?
            .values()
            .find(|r| r.public().remote_id.as_deref() == Some(id))
            .cloned())
    }

    pub fn save(&self, record: &Record) -> AppResult<()> {
        if !valid_id(&record.id) {
            return Err("invalid_job_id".into());
        }
        let bytes = serde_json::to_vec_pretty(record).map_err(|_| "save_failed")?;
        atomic_write(
            &self.root.join("jobs").join(format!("{}.json", record.id)),
            &bytes,
        )?;
        self.records
            .lock()
            .map_err(|_| "save_failed")?
            .insert(record.id.clone(), record.clone());
        Ok(())
    }

    pub fn list(&self) -> AppResult<Vec<Job>> {
        let mut records: Vec<_> = self
            .records
            .lock()
            .map_err(|_| "save_failed")?
            .values()
            .map(Record::public)
            .collect();
        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        records.truncate(200);
        Ok(records)
    }

    pub fn image_path(&self, path: &Path) -> AppResult<PathBuf> {
        let path = path.canonicalize().map_err(|_| "download_failed")?;
        let root = self
            .root
            .join("outputs")
            .canonicalize()
            .map_err(|_| "save_failed")?;
        if !path.starts_with(root) || path.extension().and_then(|s| s.to_str()) != Some("png") {
            return Err("download_failed".into());
        }
        Ok(path)
    }
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or("save_failed")?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| "save_failed")?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| "save_failed")?;
    let saved = file.persist(path).map_err(|_| "save_failed")?;
    saved.sync_all().map_err(|_| "save_failed")?;
    // Make the renamed unlock intent durable before a paid request, including
    // the directory entry on systems that support syncing directories.
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| "save_failed")?;
    Ok(())
}

pub(crate) fn valid_id(id: &str) -> bool {
    id.strip_prefix("r_")
        .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
}

/// Old paid originals remain available, but a legacy locked result grants no new spending authority.
fn legacy(bytes: &[u8]) -> Option<Record> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let job = value.get("job")?;
    let old_id = job["id"].as_str()?.strip_prefix("c_")?;
    let uuid = uuid::Uuid::parse_str(old_id).ok()?;
    let remote = serde_json::from_value::<RemoteTask>(value["remote"].clone()).ok();
    let state = if job["status"] == "completed" {
        WorkState::Complete {
            remote,
            path: PathBuf::from(job["imagePath"].as_str()?),
        }
    } else {
        WorkState::Failed {
            code: "legacy_locked".into(),
            remote,
        }
    };
    Some(Record {
        schema: 4,
        id: format!("r_{uuid}"),
        description: job["description"].as_str().unwrap_or("").into(),
        created_at: job["createdAt"].as_str().unwrap_or("").into(),
        consent: Consent::ReadOnly,
        state,
        last_error: None,
    })
}
