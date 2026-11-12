mod imaging;
mod prompts;
mod protocol;
mod store;
mod types;

pub use prompts::reference_bytes;
pub use types::{AppResult, CHARACTER_IDS, GenerateRequest, Job, Progress, Quote, Stage};

use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use types::{Consent, Record, RemoteTask, WorkState, check_keep, valid_remote_id};

/// Decode a bounded local file on the worker and return an orientation-correct
/// PNG of at most 240 pixels per side. Small images retain their original size.
pub fn thumbnail(path: &Path) -> AppResult<Vec<u8>> {
    imaging::thumbnail(path)
}

/// Blocking worker API. Construct and call it on a background thread, never from a frame callback.
#[derive(Clone)]
pub struct Engine {
    store: Arc<store::Store>,
    operation: Arc<Mutex<()>>,
}

impl Engine {
    pub fn new(data_dir: PathBuf) -> AppResult<Self> {
        Ok(Self {
            store: Arc::new(store::Store::open(data_dir)?),
            operation: Arc::new(Mutex::new(())),
        })
    }

    pub fn has_embedded_key() -> bool {
        !protocol::configured_key().is_empty()
    }

    pub fn list(&self) -> AppResult<Vec<Job>> {
        let mut jobs = self.store.list()?;
        for job in &mut jobs {
            if job
                .image_path
                .as_ref()
                .is_some_and(|path| self.store.image_path(path).is_err())
            {
                job.image_path = None;
            }
        }
        Ok(jobs)
    }

    pub fn quote(&self, request: &GenerateRequest) -> AppResult<Quote> {
        let mut request = request.clone();
        if request.mode == "text" && request.description.trim().is_empty() {
            request.description = "A gentle everyday moment.".into();
        }
        request.validate()?;
        let (prompt, count) = prompts::quote_input(&request)?;
        Ok(Quote {
            credits: protocol::RemoteClient::new()?.quote(&prompt, count, &request.resolution)?,
        })
    }

    pub fn generate(
        &self,
        request: &GenerateRequest,
        mut progress: impl FnMut(Progress),
    ) -> AppResult<Job> {
        let _guard = self
            .operation
            .try_lock()
            .map_err(|_| "generation_in_progress")?;
        request.validate()?;
        let max_cost = request
            .max_cost
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or("unknown_price")?;
        let mut provider = protocol::RemoteClient::new()?;
        let mut record = Record {
            schema: 4,
            id: format!("r_{}", uuid::Uuid::new_v4()),
            description: request.description.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            consent: Consent::Automatic { max_cost },
            state: WorkState::Preparing,
            last_error: None,
        };
        self.store.save(&record)?;
        progress(record.public());
        let prepared = prompts::prepare(request);
        let (prompt, references) = match prepared {
            Ok(value) => value,
            Err(error) => {
                record.state = WorkState::Failed {
                    code: error.clone(),
                    remote: None,
                };
                self.store.save(&record)?;
                progress(record.public());
                return Err(error);
            }
        };
        let client_ref = uuid::Uuid::new_v4().to_string();
        record.state = WorkState::Submitting {
            client_ref: client_ref.clone(),
        };
        self.store.save(&record)?;
        progress(record.public());
        match provider.submit(
            &prompt,
            &references,
            &request.aspect_ratio,
            &request.resolution,
            true,
            &client_ref,
            max_cost,
        ) {
            Ok(remote) => record.state = WorkState::Submitted { remote },
            Err(error) => {
                record.last_error = Some(error.clone());
                self.store.save(&record)?;
                progress(record.public());
                return Err(error);
            }
        }
        self.store.save(&record)?;
        self.run_record(&mut record, &mut provider, &mut progress)
    }

    pub fn resume(&self, local_id: &str, mut progress: impl FnMut(Progress)) -> AppResult<Job> {
        let _guard = self
            .operation
            .try_lock()
            .map_err(|_| "generation_in_progress")?;
        let mut record = self.store.read(local_id)?;
        let mut provider = protocol::RemoteClient::new()?;
        self.run_record(&mut record, &mut provider, &mut progress)
    }

    /// Importing an ID never grants spending authority. Existing automatic jobs retain their ledger.
    pub fn restore(&self, remote_id: &str, mut progress: impl FnMut(Progress)) -> AppResult<Job> {
        let _guard = self
            .operation
            .try_lock()
            .map_err(|_| "generation_in_progress")?;
        if !valid_remote_id(remote_id) {
            return Err("invalid_job_id".into());
        }
        let mut record = if let Some(record) = self.store.find_remote(remote_id)? {
            record
        } else {
            Record {
                schema: 4,
                id: format!("r_{}", uuid::Uuid::new_v4()),
                description: String::new(),
                created_at: chrono::Utc::now().to_rfc3339(),
                consent: Consent::ReadOnly,
                last_error: None,
                state: WorkState::Submitted {
                    remote: RemoteTask {
                        job_id: remote_id.into(),
                        gen_id: None,
                        status: "queued".into(),
                        url: None,
                        cost: None,
                    },
                },
            }
        };
        self.store.save(&record)?;
        let mut provider = protocol::RemoteClient::new()?;
        self.run_record(&mut record, &mut provider, &mut progress)
    }

    pub fn load_image(&self, local_id: &str) -> AppResult<Vec<u8>> {
        let record = self.store.read(local_id)?;
        let WorkState::Complete { path, .. } = &record.state else {
            return Err("result_not_ready".into());
        };
        let path = self.store.image_path(path)?;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| "download_failed")?
            .take((imaging::MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "download_failed")?;
        imaging::decode(&bytes)?;
        Ok(bytes)
    }

    pub fn export(&self, local_id: &str, destination: &Path) -> AppResult<()> {
        store::atomic_write(destination, &self.load_image(local_id)?)
    }

    fn run_record(
        &self,
        record: &mut Record,
        provider: &mut impl Provider,
        progress: &mut impl FnMut(Progress),
    ) -> AppResult<Job> {
        record.last_error = None;
        let result = self.drive(record, provider, progress);
        if let Err(error) = &result {
            record.last_error = Some(error.clone());
            self.store.save(record)?;
            progress(record.public());
        }
        result
    }

    fn drive(
        &self,
        record: &mut Record,
        provider: &mut impl Provider,
        progress: &mut impl FnMut(Progress),
    ) -> AppResult<Job> {
        let deadline = Instant::now() + Duration::from_secs(600);
        loop {
            progress(record.public());
            if Instant::now() >= deadline {
                return Err("waiting_paused".into());
            }
            match record.state.clone() {
                WorkState::Preparing => return Err("interrupted".into()),
                WorkState::Submitting { .. } => return Err("submit_uncertain".into()),
                WorkState::Failed { code, remote: None } => return Err(code),
                WorkState::Failed {
                    remote: Some(remote),
                    ..
                } => {
                    record.state = WorkState::Submitted { remote };
                    self.store.save(record)?;
                }
                WorkState::Submitted { remote } => {
                    let remote = provider.status(&remote)?;
                    record.state = WorkState::Submitted {
                        remote: remote.clone(),
                    };
                    self.store.save(record)?;
                    match remote.status.as_str() {
                        "queued" | "running" => {
                            // The service wait usually blocks. Bound fast queued replies as well.
                            std::thread::sleep(Duration::from_millis(300));
                        }
                        "failed" => {
                            record.state = WorkState::Failed {
                                code: "generation_failed".into(),
                                remote: Some(remote),
                            };
                            self.store.save(record)?;
                            return Err("generation_failed".into());
                        }
                        "completed" => {
                            record.state = WorkState::Unlocked { remote };
                            self.store.save(record)?;
                        }
                        "preview" => {
                            let Consent::Automatic { max_cost } = record.consent else {
                                return Err("original_locked".into());
                            };
                            check_keep(&remote, max_cost)?;
                            // Durable intent BEFORE the paid call: a crash can never cause its replay.
                            record.state = WorkState::Unlocking {
                                remote: remote.clone(),
                            };
                            self.store.save(record)?;
                            progress(record.public());
                            let unlocked = provider.keep(&remote, max_cost)?;
                            if unlocked.status != "completed" {
                                return Err("keep_uncertain".into());
                            }
                            record.state = WorkState::Unlocked { remote: unlocked };
                            self.store.save(record)?;
                        }
                        _ => return Err("provider_response".into()),
                    }
                }
                WorkState::Unlocking { remote } => {
                    // Only a read is allowed after a previous unlock attempt, even after a restart.
                    let latest = provider.status(&remote)?;
                    if latest.status != "completed" {
                        return Err("keep_uncertain".into());
                    }
                    record.state = WorkState::Unlocked { remote: latest };
                    self.store.save(record)?;
                }
                WorkState::Unlocked { remote } => {
                    if remote.status != "completed" {
                        return Err("original_locked".into());
                    }
                    let bytes =
                        provider.download(remote.url.as_deref().ok_or("download_failed")?)?;
                    imaging::decode(&bytes)?;
                    let path = self
                        .store
                        .root
                        .join("outputs")
                        .join(format!("chiikawa_{}.png", record.id));
                    store::atomic_write(&path, &bytes)?;
                    record.state = WorkState::Complete {
                        remote: Some(remote),
                        path,
                    };
                    self.store.save(record)?;
                }
                WorkState::Complete { remote, path } => {
                    if self.store.image_path(&path).is_ok() {
                        record.last_error = None;
                        self.store.save(record)?;
                        progress(record.public());
                        return Ok(record.public());
                    }
                    let remote = remote.ok_or("download_failed")?;
                    if remote.status != "completed" {
                        return Err("original_locked".into());
                    }
                    record.state = WorkState::Unlocked { remote };
                    self.store.save(record)?;
                }
            }
        }
    }
}

trait Provider {
    fn status(&mut self, task: &RemoteTask) -> AppResult<RemoteTask>;
    fn keep(&mut self, task: &RemoteTask, cap: f64) -> AppResult<RemoteTask>;
    fn download(&mut self, url: &str) -> AppResult<Vec<u8>>;
}

impl Provider for protocol::RemoteClient {
    fn status(&mut self, task: &RemoteTask) -> AppResult<RemoteTask> {
        self.status(task)
    }
    fn keep(&mut self, task: &RemoteTask, cap: f64) -> AppResult<RemoteTask> {
        self.keep(task, cap)
    }
    fn download(&mut self, url: &str) -> AppResult<Vec<u8>> {
        protocol::download(url)
    }
}

#[cfg(test)]
mod tests;
