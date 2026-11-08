use serde::{Deserialize, Serialize};

pub type AppResult<T> = Result<T, String>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub provider: String,
    pub mode: String,
    pub description: String,
    #[serde(default)]
    pub photos: Vec<String>,
    pub custom_reference: Option<String>,
    pub character: String,
    pub background: String,
    pub emotion: String,
    pub strength: f32,
    #[serde(default)]
    pub full_creature: bool,
    pub aspect_ratio: String,
    pub resolution: String,
    #[serde(default)]
    pub credit_mode: bool,
    #[serde(default = "yes")]
    pub clean_codex: bool,
}

fn yes() -> bool {
    true
}

impl GenerateRequest {
    pub fn validate(&self) -> AppResult<()> {
        if !["elyum", "codex"].contains(&self.provider.as_str())
            || !["text", "photo"].contains(&self.mode.as_str())
            || !["own", "chiikawa", "hachiware", "usagi", "trio"].contains(&self.character.as_str())
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
        if self.mode == "photo" && (self.photos.is_empty() || self.photos.len() > 8) {
            return Err("photos_required".into());
        }
        if self.photos.len() > 8 {
            return Err("too_many_photos".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub provider: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTask {
    pub job_id: String,
    pub gen_id: Option<String>,
    pub status: String,
    pub url: Option<String>,
    pub cost: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub job: Job,
    pub remote: Option<RemoteTask>,
    pub downloaded_status: Option<String>,
    pub client_ref: String,
}

pub fn valid_remote_id(id: &str) -> bool {
    id.starts_with("g_")
        && (3..=150).contains(&id.len())
        && id[2..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn check_keep(task: &RemoteTask, confirmed_cost: f64) -> AppResult<bool> {
    if task.status == "completed" {
        return Ok(false);
    }
    if task.status != "preview" {
        return Err("preview_not_ready".into());
    }
    if !task.gen_id.as_ref().is_some_and(|id| {
        id.starts_with("g_") && id.len() > 2 && id[2..].bytes().all(|b| b.is_ascii_hexdigit())
    }) {
        return Err("preview_not_ready".into());
    }
    let cost = task
        .cost
        .filter(|c| c.is_finite() && *c >= 0.0)
        .ok_or("unknown_price")?;
    if !confirmed_cost.is_finite()
        || confirmed_cost < 0.0
        || (confirmed_cost - cost).abs() > 0.000001
    {
        return Err("price_changed".into());
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task() -> RemoteTask {
        RemoteTask {
            job_id: "g_abcdef".into(),
            gen_id: Some("g_abcdef".into()),
            status: "preview".into(),
            url: None,
            cost: Some(6.0),
        }
    }
    #[test]
    fn keep_requires_same_known_price() {
        assert_eq!(check_keep(&task(), 6.0), Ok(true));
        assert_eq!(check_keep(&task(), 9.0), Err("price_changed".into()));
        let mut t = task();
        t.cost = None;
        assert_eq!(check_keep(&t, 6.0), Err("unknown_price".into()));
        t.cost = Some(f64::NAN);
        assert_eq!(check_keep(&t, 6.0), Err("unknown_price".into()));
    }
    #[test]
    fn paid_job_skips_keep_even_when_old_price_changed() {
        let mut t = task();
        t.status = "completed".into();
        t.cost = Some(9.0);
        assert_eq!(check_keep(&t, 6.0), Ok(false));
    }
    #[test]
    fn queued_job_cannot_be_paid() {
        let mut t = task();
        t.status = "running".into();
        assert_eq!(check_keep(&t, 6.0), Err("preview_not_ready".into()));
    }
}
