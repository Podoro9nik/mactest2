use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub type AppResult<T> = Result<T, String>;

pub const CHARACTER_IDS: &[&str] = &[
    "own",
    "trio",
    "chiikawa",
    "hachiware",
    "usagi",
    "momonga",
    "rakko",
    "kurimanju",
    "shisa",
    "furuhonya",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub mode: String,
    pub description: String,
    pub photos: Vec<PathBuf>,
    pub custom_reference: Option<PathBuf>,
    pub character: String,
    pub background: String,
    pub emotion: String,
    pub strength: f32,
    pub full_creature: bool,
    pub aspect_ratio: String,
    pub resolution: String,
    /// The known price shown before the Generate click. Never charge more than this.
    pub max_cost: Option<f64>,
}

impl Default for GenerateRequest {
    fn default() -> Self {
        Self {
            mode: "text".into(),
            description: String::new(),
            photos: Vec::new(),
            custom_reference: None,
            character: "own".into(),
            background: "scene".into(),
            emotion: "scene".into(),
            strength: 50.0,
            full_creature: false,
            aspect_ratio: "1:1".into(),
            resolution: "2K".into(),
            max_cost: None,
        }
    }
}

impl GenerateRequest {
    pub fn validate(&self) -> AppResult<()> {
        if !["text", "photo"].contains(&self.mode.as_str())
            || !CHARACTER_IDS.contains(&self.character.as_str())
            || !["scene", "white", "meadow", "room", "sticker"].contains(&self.background.as_str())
            || !["scene", "happy", "shy", "tears", "surprised"].contains(&self.emotion.as_str())
            || !["1:1", "3:4", "4:3", "9:16", "16:9"].contains(&self.aspect_ratio.as_str())
            || !["1K", "2K", "4K"].contains(&self.resolution.as_str())
            || !self.strength.is_finite()
            || !(0.0..=100.0).contains(&self.strength)
        {
            return Err("invalid_settings".into());
        }
        if self.description.chars().count() > 1200 {
            return Err("description_too_long".into());
        }
        if self.mode == "text" && self.description.trim().is_empty() {
            return Err("description_required".into());
        }
        if self.mode == "photo" && self.photos.is_empty() {
            return Err("photos_required".into());
        }
        if self.photos.len() > 8 {
            return Err("too_many_photos".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Preparing,
    Uploading,
    Generating,
    Unlocking,
    Downloading,
    Completed,
    Failed,
}

impl Stage {
    pub fn key(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Uploading => "uploading",
            Self::Generating => "generating",
            Self::Unlocking => "unlocking",
            Self::Downloading => "downloading",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub description: String,
    pub created_at: String,
    pub stage: Stage,
    pub remote_id: Option<String>,
    pub cost: Option<f64>,
    /// Always a clean, unlocked original. Locked media is never exposed by this API.
    pub image_path: Option<PathBuf>,
    pub error: Option<String>,
}

pub type Progress = Job;

#[derive(Clone, Debug)]
pub struct Quote {
    pub credits: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteTask {
    pub job_id: String,
    pub gen_id: Option<String>,
    pub status: String,
    pub url: Option<String>,
    pub cost: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub(crate) enum Consent {
    ReadOnly,
    Automatic { max_cost: f64 },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub(crate) enum WorkState {
    Preparing,
    Submitting {
        client_ref: String,
    },
    Submitted {
        remote: RemoteTask,
    },
    /// Written before the one paid request. Re-entry can only read its result.
    Unlocking {
        remote: RemoteTask,
    },
    Unlocked {
        remote: RemoteTask,
    },
    Complete {
        remote: Option<RemoteTask>,
        path: PathBuf,
    },
    Failed {
        code: String,
        remote: Option<RemoteTask>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Record {
    pub schema: u8,
    pub id: String,
    pub description: String,
    pub created_at: String,
    pub consent: Consent,
    pub state: WorkState,
    pub last_error: Option<String>,
}

impl Record {
    pub fn public(&self) -> Job {
        let (stage, remote, path, state_error) = match &self.state {
            WorkState::Preparing => (Stage::Preparing, None, None, None),
            WorkState::Submitting { .. } => (Stage::Uploading, None, None, None),
            WorkState::Submitted { remote } => (Stage::Generating, Some(remote), None, None),
            WorkState::Unlocking { remote } => (Stage::Unlocking, Some(remote), None, None),
            WorkState::Unlocked { remote } => (Stage::Downloading, Some(remote), None, None),
            WorkState::Complete { remote, path } => {
                (Stage::Completed, remote.as_ref(), Some(path.clone()), None)
            }
            WorkState::Failed { code, remote } => {
                (Stage::Failed, remote.as_ref(), None, Some(code.clone()))
            }
        };
        Job {
            id: self.id.clone(),
            description: self.description.clone(),
            created_at: self.created_at.clone(),
            stage: if self.last_error.is_some() {
                Stage::Failed
            } else {
                stage
            },
            remote_id: remote.map(|r| r.job_id.clone()),
            cost: remote.and_then(|r| r.cost),
            image_path: path,
            error: state_error.or_else(|| self.last_error.clone()),
        }
    }
}

pub(crate) fn valid_remote_id(id: &str) -> bool {
    id.starts_with("g_")
        && (3..=150).contains(&id.len())
        && id[2..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub(crate) fn check_keep(task: &RemoteTask, cap: f64) -> AppResult<bool> {
    if task.status == "completed" {
        return Ok(false);
    }
    if task.status != "preview" {
        return Err("result_not_ready".into());
    }
    if !task.gen_id.as_ref().is_some_and(|id| valid_remote_id(id)) {
        return Err("result_not_ready".into());
    }
    let price = task
        .cost
        .filter(|c| c.is_finite() && *c >= 0.0)
        .ok_or("unknown_price")?;
    if !cap.is_finite() || cap < 0.0 {
        return Err("unknown_price".into());
    }
    if price > cap + 0.000001 {
        return Err("price_changed".into());
    }
    Ok(true)
}
