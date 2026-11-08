use crate::{
    imaging,
    types::{AppResult, Job, Record, RemoteTask},
};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub struct Store {
    pub root: PathBuf,
    pub resources: PathBuf,
    records: Mutex<HashMap<String, Record>>,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl Store {
    pub fn open(root: PathBuf, resources: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(root.join("jobs")).map_err(|_| "save_failed")?;
        std::fs::create_dir_all(root.join("outputs")).map_err(|_| "save_failed")?;
        let mut records = HashMap::new();
        for entry in std::fs::read_dir(root.join("jobs"))
            .map_err(|_| "save_failed")?
            .flatten()
        {
            if entry.path().extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            let Ok(mut record) = serde_json::from_slice::<Record>(&bytes) else {
                continue;
            };
            if !valid_local_id(&record.job.id) {
                continue;
            }
            record.job.image_url = None;
            if ["queued", "running"].contains(&record.job.status.as_str())
                && (record.job.provider == "codex" || record.remote.is_none())
            {
                record.job.status = "failed".into();
                record.job.error = Some(
                    if record.job.provider == "codex" {
                        "interrupted"
                    } else {
                        "submit_uncertain"
                    }
                    .into(),
                );
            }
            records.insert(record.job.id.clone(), record);
        }
        Ok(Self {
            root,
            resources,
            records: Mutex::new(records),
            locks: Mutex::new(HashMap::new()),
        })
    }

    pub fn read(&self, id: &str) -> AppResult<Record> {
        self.records
            .lock()
            .map_err(|_| "save_failed")?
            .get(id)
            .cloned()
            .ok_or_else(|| "job_not_found".into())
    }

    pub fn save(&self, record: &Record) -> AppResult<()> {
        if !valid_local_id(&record.job.id) {
            return Err("invalid_job_id".into());
        }
        let mut persisted = record.clone();
        persisted.job.image_url = None;
        let bytes = serde_json::to_vec_pretty(&persisted).map_err(|_| "save_failed")?;
        atomic_write(
            &self
                .root
                .join("jobs")
                .join(format!("{}.json", record.job.id)),
            &bytes,
        )?;
        self.records
            .lock()
            .map_err(|_| "save_failed")?
            .insert(record.job.id.clone(), persisted);
        Ok(())
    }

    pub fn lock(&self, id: &str) -> AppResult<Arc<Mutex<()>>> {
        let mut locks = self.locks.lock().map_err(|_| "save_failed")?;
        Ok(locks
            .entry(id.into())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone())
    }

    pub fn list(&self) -> AppResult<Vec<Job>> {
        let records = self.records.lock().map_err(|_| "save_failed")?;
        let mut jobs: Vec<Job> = records.values().map(|r| r.job.clone()).collect();
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        jobs.truncate(200);
        Ok(jobs)
    }

    pub fn public(&self, record: &Record) -> AppResult<Job> {
        let mut job = record.job.clone();
        job.image_url = None;
        if let Some(path) = &job.image_path {
            if let Ok(safe) = self.image_path(path) {
                if let Ok(bytes) = std::fs::read(safe) {
                    if bytes.len() <= imaging::MAX_BYTES {
                        job.image_url = Some(imaging::data_url(&bytes));
                    }
                }
            }
        }
        Ok(job)
    }

    pub fn image_path(&self, path: &str) -> AppResult<PathBuf> {
        let path = Path::new(path)
            .canonicalize()
            .map_err(|_| "download_failed")?;
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

    pub fn fail(&self, id: &str, error: &str, terminal: bool) {
        if let Ok(mut record) = self.read(id) {
            record.job.error = Some(error.into());
            if terminal {
                record.job.status = "failed".into();
            }
            let _ = self.save(&record);
        }
    }

    pub fn save_image(&self, record: &mut Record, bytes: &[u8], status: &str) -> AppResult<()> {
        let prefix = if status == "preview" {
            "preview"
        } else {
            "chiikawa"
        };
        let path = self
            .root
            .join("outputs")
            .join(format!("{prefix}_{}.png", record.job.id));
        atomic_write(&path, bytes)?;
        record.job.image_path = Some(path.to_string_lossy().into());
        record.downloaded_status = Some(status.into());
        record.job.error = None;
        self.save(record)
    }
}

pub fn apply_remote(record: &mut Record, task: RemoteTask) {
    if record.downloaded_status.as_deref() != Some(&task.status) {
        record.job.image_path = None;
        record.job.image_url = None;
    }
    record.job.status = task.status.clone();
    record.job.cost = task.cost;
    record.job.remote_id = Some(task.job_id.clone());
    record.job.error = if task.status == "failed" {
        Some("generation_failed".into())
    } else {
        None
    };
    record.remote = Some(task);
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or("save_failed")?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| "save_failed")?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| "save_failed")?;
    file.persist(path).map_err(|_| "save_failed")?;
    Ok(())
}

fn valid_local_id(id: &str) -> bool {
    id.strip_prefix("c_")
        .is_some_and(|s| uuid::Uuid::parse_str(s).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> Record {
        Record {
            job: Job {
                id: format!("c_{}", uuid::Uuid::new_v4()),
                provider: "elyum".into(),
                status: "preview".into(),
                image_url: None,
                image_path: Some("old.png".into()),
                cost: Some(6.0),
                error: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                description: "test".into(),
                remote_id: Some("g_abc".into()),
            },
            remote: None,
            downloaded_status: Some("preview".into()),
            client_ref: uuid::Uuid::new_v4().to_string(),
        }
    }
    #[test]
    fn completed_metadata_replaces_preview_before_download() {
        let mut r = record();
        apply_remote(
            &mut r,
            RemoteTask {
                job_id: "g_abc".into(),
                gen_id: Some("g_abc".into()),
                status: "completed".into(),
                url: Some("https://elyum.ai/media/original".into()),
                cost: Some(6.0),
            },
        );
        assert_eq!(r.job.status, "completed");
        assert!(r.job.image_path.is_none());
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path().into(), dir.path().into()).unwrap();
        s.save(&r).unwrap();
        let reopened = Store::open(dir.path().into(), dir.path().into()).unwrap();
        assert_eq!(
            reopened.read(&r.job.id).unwrap().remote.unwrap().status,
            "completed"
        );
    }
    #[test]
    fn atomic_replace_preserves_last_state() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.json");
        atomic_write(&p, b"first").unwrap();
        atomic_write(&p, b"second").unwrap();
        assert_eq!(std::fs::read(p).unwrap(), b"second");
    }
    #[test]
    fn cannot_export_outside_output_directory() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path().into(), dir.path().into()).unwrap();
        std::fs::write(dir.path().join("secret.png"), b"secret").unwrap();
        assert!(s
            .image_path(&dir.path().join("secret.png").to_string_lossy())
            .is_err());
    }
}
