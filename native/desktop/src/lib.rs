mod codex;
mod elyum;
mod imaging;
mod prompts;
mod store;
mod types;

use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::Manager;
use types::{AppResult, GenerateRequest, Job, Record, RemoteTask};

#[derive(Clone)]
struct AppState {
    store: Arc<store::Store>,
    login_pending: Arc<AtomicBool>,
    submit_lock: Arc<Mutex<()>>,
    smoke_report: Option<PathBuf>,
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| "internal_error".to_string())?
}

#[tauri::command]
fn app_info(state: tauri::State<'_, AppState>) -> Value {
    json!({"version":env!("CARGO_PKG_VERSION"),"outputDir":state.store.root.join("outputs"),"hasEmbeddedKey":!elyum::configured_key().is_empty(),"codexAvailable":codex::find_codex(&state.store.resources).is_some(),"smokeTest":state.smoke_report.is_some()})
}

#[tauri::command]
async fn account(state: tauri::State<'_, AppState>) -> AppResult<Value> {
    if state.smoke_report.is_some() {
        return Err("smoke_network_disabled".into());
    }
    blocking(|| elyum::Elyum::new()?.account()).await
}

#[tauri::command]
async fn list_jobs(state: tauri::State<'_, AppState>) -> AppResult<Vec<Job>> {
    let store = state.store.clone();
    blocking(move || store.list()).await
}

fn new_record(provider: &str, description: &str) -> Record {
    Record {
        job: Job {
            id: format!("c_{}", uuid::Uuid::new_v4()),
            provider: provider.into(),
            status: "queued".into(),
            image_url: None,
            image_path: None,
            cost: None,
            error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            description: description.into(),
            remote_id: None,
        },
        remote: None,
        downloaded_status: None,
        client_ref: uuid::Uuid::new_v4().to_string(),
    }
}

#[tauri::command]
async fn generate(request: GenerateRequest, state: tauri::State<'_, AppState>) -> AppResult<Job> {
    request.validate()?;
    if state.smoke_report.is_some() {
        return Err("smoke_network_disabled".into());
    }
    let state = state.inner().clone();
    let record = blocking({
        let state = state.clone();
        let provider = request.provider.clone();
        let description = request.description.clone();
        move || {
            let _guard = state.submit_lock.lock().map_err(|_| "internal_error")?;
            if state.store.list()?.iter().any(|j| {
                j.provider == provider && ["queued", "running"].contains(&j.status.as_str())
            }) {
                return Err("generation_in_progress".into());
            }
            let record = new_record(&provider, &description);
            state.store.save(&record)?;
            Ok(record)
        }
    })
    .await?;
    let public = record.job.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let id = record.job.id.clone();
        if let Err(error) = run_generation(&state.store, record, request) {
            state.store.fail(&id, &error, true);
        }
    });
    Ok(public)
}

fn run_generation(
    store: &store::Store,
    mut record: Record,
    request: GenerateRequest,
) -> AppResult<()> {
    let lock = store.lock(&record.job.id)?;
    let _guard = lock.lock().map_err(|_| "internal_error")?;
    record.job.status = "running".into();
    store.save(&record)?;
    let (prompt, references) = prompts::prepare(&request)?;
    if request.provider == "elyum" {
        let task = elyum::Elyum::new()?.submit(
            &prompt,
            &references,
            &request.aspect_ratio,
            &request.resolution,
            request.credit_mode,
            &record.client_ref,
        )?;
        store::apply_remote(&mut record, task);
        store.save(&record)?;
    } else {
        let directory = store.root.join("codex-sessions").join(&record.job.id);
        let mut session = codex::Session::start(&store.resources, &directory)?;
        let bytes = session.generate(&prompt, &references, &request.aspect_ratio)?;
        let bytes = if request.clean_codex {
            let original = store
                .root
                .join("outputs")
                .join(format!("codex_original_{}.png", record.job.id));
            store::atomic_write(&original, &bytes)?;
            imaging::png_bytes(&imaging::clean_flat_art(&imaging::decode(&bytes)?))?
        } else {
            bytes
        };
        record.job.status = "completed".into();
        store.save_image(&mut record, &bytes, "completed")?;
    }
    Ok(())
}

fn download_ready(store: &store::Store, record: &mut Record) -> AppResult<()> {
    let task = record.remote.as_ref().ok_or("job_not_found")?;
    if !["preview", "completed"].contains(&task.status.as_str()) {
        return Ok(());
    }
    if record.downloaded_status.as_deref() == Some(&task.status)
        && record
            .job
            .image_path
            .as_ref()
            .is_some_and(|p| store.image_path(p).is_ok())
    {
        return Ok(());
    }
    let status = task.status.clone();
    let bytes = elyum::download(task.url.as_deref().ok_or("download_failed")?)?;
    store.save_image(record, &bytes, &status)
}

#[tauri::command]
async fn get_job(job_id: String, state: tauri::State<'_, AppState>) -> AppResult<Job> {
    if state.smoke_report.is_some() {
        return Err("smoke_network_disabled".into());
    }
    let store = state.store.clone();
    blocking(move || {
        let lock = store.lock(&job_id)?;
        let Ok(_guard) = lock.try_lock() else {
            return store.public(&store.read(&job_id)?);
        };
        let mut record = store.read(&job_id)?;
        if record.job.provider != "elyum" || record.remote.is_none() {
            return store.public(&record);
        }
        if record.job.status == "completed"
            && record.downloaded_status.as_deref() == Some("completed")
            && record
                .job
                .image_path
                .as_ref()
                .is_some_and(|p| store.image_path(p).is_ok())
        {
            return store.public(&record);
        }
        let result = (|| {
            let task = elyum::Elyum::new()?.status(record.remote.as_ref().unwrap())?;
            store::apply_remote(&mut record, task);
            // Persist unlock state before downloading, so failed downloads never cause a second payment.
            store.save(&record)?;
            download_ready(&store, &mut record)
        })();
        if let Err(error) = result {
            record.job.error = Some(error);
            store.save(&record)?;
        }
        store.public(&record)
    })
    .await
}

#[tauri::command]
async fn restore_job(job_id: String, state: tauri::State<'_, AppState>) -> AppResult<Job> {
    if !types::valid_remote_id(job_id.trim()) {
        return Err("invalid_job_id".into());
    }
    if state.smoke_report.is_some() {
        return Err("smoke_network_disabled".into());
    }
    let store = state.store.clone();
    let id = job_id.trim().to_string();
    blocking(move || {
        if let Some(existing) = store
            .list()?
            .into_iter()
            .find(|j| j.remote_id.as_deref() == Some(&id))
        {
            return store.public(&store.read(&existing.id)?);
        }
        let mut record = new_record("elyum", "");
        let task = RemoteTask {
            job_id: id,
            gen_id: None,
            status: "queued".into(),
            url: None,
            cost: None,
        };
        store::apply_remote(&mut record, task.clone());
        store.save(&record)?;
        let task = elyum::Elyum::new()?.status(&task)?;
        store::apply_remote(&mut record, task);
        store.save(&record)?;
        if let Err(error) = download_ready(&store, &mut record) {
            record.job.error = Some(error);
            store.save(&record)?;
        }
        store.public(&record)
    })
    .await
}

#[tauri::command]
async fn keep_original(
    job_id: String,
    confirmed_cost: f64,
    state: tauri::State<'_, AppState>,
) -> AppResult<Job> {
    if state.smoke_report.is_some() {
        return Err("smoke_network_disabled".into());
    }
    let store = state.store.clone();
    blocking(move || {
        let lock = store.lock(&job_id)?;
        let _guard = lock.lock().map_err(|_| "internal_error")?;
        let mut record = store.read(&job_id)?;
        if record.job.provider != "elyum" {
            return Err("invalid_settings".into());
        }
        let mut client = elyum::Elyum::new()?;
        let refreshed = client.status(record.remote.as_ref().ok_or("preview_not_ready")?)?;
        store::apply_remote(&mut record, refreshed.clone());
        store.save(&record)?;
        // Compare against the exact price shown in the user's confirmation dialog.
        if types::check_keep(&refreshed, confirmed_cost)? {
            if record.downloaded_status.as_deref() != Some("preview")
                || record.job.image_path.is_none()
            {
                return Err("preview_not_ready".into());
            }
            let kept = match client.keep(&refreshed, confirmed_cost) {
                Ok(t) => t,
                Err(e) => {
                    record.job.error = Some(e.clone());
                    store.save(&record)?;
                    return Err(e);
                }
            };
            store::apply_remote(&mut record, kept);
            store.save(&record)?;
        }
        if record.job.status != "completed" {
            return Err("preview_not_ready".into());
        }
        if let Err(error) = download_ready(&store, &mut record) {
            record.job.error = Some(error.clone());
            store.save(&record)?;
            return Err(error);
        }
        store.public(&record)
    })
    .await
}

#[tauri::command]
async fn export_image(
    job_id: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Option<String>> {
    let store = state.store.clone();
    blocking(move || {
        let record = store.read(&job_id)?;
        let source =
            store.image_path(record.job.image_path.as_deref().ok_or("download_failed")?)?;
        let name = source
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("save_failed")?;
        let target = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .set_file_name(name)
            .save_file();
        if let Some(target) = target {
            if target != source {
                std::fs::copy(&source, &target).map_err(|_| "save_failed")?;
            }
            return Ok(Some(target.to_string_lossy().into()));
        }
        Ok(None)
    })
    .await
}

#[tauri::command]
async fn codex_account(state: tauri::State<'_, AppState>) -> AppResult<Value> {
    if state.smoke_report.is_some() {
        return Err("smoke_network_disabled".into());
    }
    let store = state.store.clone();
    blocking(move || {
        let directory = tempfile::Builder::new()
            .prefix("account-")
            .tempdir_in(&store.root)
            .map_err(|_| "save_failed")?;
        codex::Session::start(&store.resources, directory.path())?.account()
    })
    .await
}

#[tauri::command]
async fn codex_login(state: tauri::State<'_, AppState>) -> AppResult<Value> {
    if state.smoke_report.is_some() {
        return Err("smoke_network_disabled".into());
    }
    if state.login_pending.swap(true, Ordering::AcqRel) {
        return Err("login_in_progress".into());
    }
    let state = state.inner().clone();
    let prepared = blocking({
        let state = state.clone();
        move || {
            let directory = tempfile::Builder::new()
                .prefix("login-")
                .tempdir_in(&state.store.root)
                .map_err(|_| "save_failed")?;
            let mut session = codex::Session::start(&state.store.resources, directory.path())?;
            let url = session.login()?;
            let opened = webbrowser::open(&url).is_ok();
            Ok((session, directory, url, opened))
        }
    })
    .await;
    match prepared {
        Ok((mut session, directory, url, opened)) => {
            tauri::async_runtime::spawn_blocking(move || {
                let _directory = directory;
                let _ = session.wait_login();
                state.login_pending.store(false, Ordering::Release);
            });
            Ok(json!({"authUrl":url,"opened":opened,"message":"login_started"}))
        }
        Err(error) => {
            state.login_pending.store(false, Ordering::Release);
            Err(error)
        }
    }
}

#[tauri::command]
fn smoke_test_complete(
    report: Value,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    let path = state.smoke_report.as_ref().ok_or("invalid_settings")?;
    let pass = prompts::reference_smoke_test().is_ok()
        && report["ok"] == true
        && report["checks"].as_object().is_some_and(|checks| {
            !checks.is_empty() && checks.values().all(|v| *v == Value::Bool(true))
        });
    let output = json!({"ok":pass,"version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"frontend":report,"references":prompts::reference_smoke_test().is_ok()});
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "save_failed")?;
    }
    store::atomic_write(
        path,
        &serde_json::to_vec_pretty(&output).map_err(|_| "save_failed")?,
    )?;
    app.exit(if pass { 0 } else { 1 });
    Ok(())
}

pub fn self_test() {
    let references = prompts::reference_smoke_test().is_ok();
    let report = json!({"ok":references,"version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"references":references,"mcpEndpoint":elyum::ENDPOINT,"hasEmbeddedKey":!elyum::configured_key().is_empty()});
    println!("{}", report);
    if !references {
        std::process::exit(1);
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let resources = app.path().resource_dir()?;
            let args: Vec<String> = std::env::args().collect();
            let smoke_report = args
                .iter()
                .position(|a| a == "--smoke-test")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from);
            let store = store::Store::open(root, resources).map_err(std::io::Error::other)?;
            app.manage(AppState {
                store: Arc::new(store),
                login_pending: Arc::new(AtomicBool::new(false)),
                submit_lock: Arc::new(Mutex::new(())),
                smoke_report,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            account,
            generate,
            get_job,
            restore_job,
            keep_original,
            list_jobs,
            export_image,
            codex_account,
            codex_login,
            smoke_test_complete
        ])
        .run(tauri::generate_context!())
        .expect("Unable to start Chiikawizator");
}
