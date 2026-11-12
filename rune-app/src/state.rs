use crate::{
    backend::{self, Engine, GenerateRequest, Job},
    i18n::Lang,
};
use rune::{Image, ImageCache, TextEdit, Tx, Waker};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Create,
    History,
    Settings,
}
pub struct Photo {
    pub path: PathBuf,
    pub name: String,
    pub image: Option<Image>,
}
enum Command {
    Quote(u64, GenerateRequest),
    Generate(GenerateRequest),
    Resume(String),
    Restore(String),
    Load(String),
    Export(String, PathBuf),
    Language(Lang),
    Thumbnail(PathBuf),
}
enum Event {
    Boot(Result<(Lang, Vec<Job>), String>),
    Quote(u64, Result<f64, String>),
    Balance(u64, Result<f64, String>),
    Progress(Job),
    Finished(Result<(Job, Vec<u8>), String>),
    History(Vec<Job>),
    Exported(Result<(), String>),
    Thumbnail(PathBuf, Result<Vec<u8>, String>),
}
pub struct State {
    pub lang: Lang,
    pub page: Page,
    pub description: TextEdit,
    pub restore_id: TextEdit,
    pub request: GenerateRequest,
    pub photos: Vec<Photo>,
    pub custom_reference: Option<Photo>,
    pub assets: HashMap<String, Image>,
    pub result: Option<Image>,
    pub job: Option<Job>,
    pub history: Vec<Job>,
    pub busy: bool,
    pub quote: Option<f64>,
    pub quote_pending: bool,
    pub balance: Option<f64>,
    pub balance_pending: bool,
    pub balance_error: Option<String>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub booted: bool,
    cache: Option<ImageCache>,
    sender: Option<Sender<Command>>,
    balance_sender: Option<Sender<u64>>,
    events: Option<Receiver<Event>>,
    waker: Option<Waker>,
    quote_id: u64,
    quote_generation: Arc<AtomicU64>,
    balance_generation: Arc<AtomicU64>,
    balance_after_job: bool,
    pub offline: bool,
}
impl State {
    pub fn new() -> Self {
        Self {
            lang: Lang::Ru,
            page: Page::Create,
            description: TextEdit::new(""),
            restore_id: TextEdit::new(""),
            request: GenerateRequest::default(),
            photos: Vec::new(),
            custom_reference: None,
            assets: HashMap::new(),
            result: None,
            job: None,
            history: Vec::new(),
            busy: false,
            quote: None,
            quote_pending: false,
            balance: None,
            balance_pending: false,
            balance_error: None,
            error: None,
            notice: None,
            booted: false,
            cache: None,
            sender: None,
            balance_sender: None,
            events: None,
            waker: None,
            quote_id: 0,
            quote_generation: Arc::new(AtomicU64::new(0)),
            balance_generation: Arc::new(AtomicU64::new(0)),
            balance_after_job: false,
            offline: std::env::var_os("CHIIKAWIZATOR_OFFLINE").is_some()
                || std::env::args().any(|a| matches!(a.as_str(), "--snap" | "--tree" | "--script")),
        }
    }
    pub fn setup(&mut self, waker: Waker) {
        let cache = ImageCache::new(waker.clone());
        for id in backend::CHARACTER_IDS {
            if let Some(bytes) = backend::reference_bytes(id) {
                self.assets.insert(id.to_string(), cache.load(bytes));
            }
        }
        if let Some(img) = self.assets.get("chiikawa").cloned() {
            self.assets.insert("own".into(), img);
        }
        self.cache = Some(cache);
        self.waker = Some(waker);
        if self.offline {
            self.booted = true;
            self.quote = Some(3.0);
            self.balance = Some(120.0);
        }
    }
    pub fn start(&mut self) {
        if self.offline || self.sender.is_some() {
            return;
        }
        let (commands_tx, commands_rx) = mpsc::channel();
        let (events_tx, events_rx) = mpsc::channel();
        let (balance_tx, balance_rx) = mpsc::channel();
        let wake = self.waker.as_ref().expect("setup before ready").clone();
        let quote_generation = self.quote_generation.clone();
        let balance_generation = self.balance_generation.clone();
        self.sender = Some(commands_tx);
        self.balance_sender = Some(balance_tx);
        self.events = Some(events_rx);
        std::thread::Builder::new()
            .name("drawing-worker".into())
            .spawn(move || {
                worker(
                    commands_rx,
                    balance_rx,
                    events_tx,
                    wake,
                    quote_generation,
                    balance_generation,
                )
            })
            .expect("start drawing worker");
    }
    fn send(&mut self, command: Command) {
        if self
            .sender
            .as_ref()
            .is_none_or(|s| s.send(command).is_err())
        {
            self.error = Some("network_error".into());
            self.busy = false;
            self.quote_pending = false;
        }
    }
    fn current_request(&self) -> GenerateRequest {
        let mut request = self.request.clone();
        request.description = self.description.text.clone();
        request.photos = self.photos.iter().map(|p| p.path.clone()).collect();
        request.custom_reference = self.custom_reference.as_ref().map(|p| p.path.clone());
        request.max_cost = self.quote;
        request
    }
    pub fn request_quote(&mut self) {
        self.quote_id += 1;
        self.quote = None;
        self.quote_generation
            .store(self.quote_id, Ordering::Relaxed);
        if self.offline {
            self.quote = Some(3.0);
            return;
        }
        if !self.booted || self.busy {
            self.quote_pending = false;
            return;
        }
        if self.request.mode == "photo" && self.photos.is_empty() {
            self.quote_pending = false;
            return;
        }
        self.quote_pending = true;
        self.send(Command::Quote(self.quote_id, self.current_request()));
    }
    pub fn set_language(&mut self, lang: Lang) {
        self.lang = lang;
        if !self.offline && self.sender.is_some() {
            self.send(Command::Language(lang));
        }
    }
    pub fn refresh_balance(&mut self) {
        if self.busy || self.balance_pending || !self.booted {
            return;
        }
        if self.offline {
            self.balance = Some(120.0);
            self.balance_error = None;
            self.balance_pending = false;
            return;
        }
        let id = self.balance_generation.fetch_add(1, Ordering::Relaxed) + 1;
        self.balance_pending = true;
        self.balance_error = None;
        if self
            .balance_sender
            .as_ref()
            .is_none_or(|sender| sender.send(id).is_err())
        {
            self.balance_pending = false;
            self.balance = None;
            self.balance_error = Some("balance_unavailable".into());
        }
    }
    fn invalidate_balance_for_job(&mut self) {
        // A balance fetched before a purchase must never replace the post-purchase value.
        self.balance_generation.fetch_add(1, Ordering::Relaxed);
        self.balance = None;
        self.balance_pending = false;
        self.balance_error = None;
        self.balance_after_job = true;
    }
    pub fn generate(&mut self) {
        if self.busy || self.offline {
            return;
        }
        let request = self.current_request();
        if let Err(error) = request.validate() {
            self.error = Some(error);
            return;
        }
        if self.quote.is_none() {
            self.error = Some("unknown_price".into());
            self.request_quote();
            return;
        }
        self.invalidate_balance_for_job();
        self.busy = true;
        self.result = None;
        self.job = None;
        self.error = None;
        self.notice = None;
        self.send(Command::Generate(request));
    }
    pub fn resume(&mut self) {
        if self.busy || self.offline {
            return;
        }
        if let Some(job) = &self.job {
            let id = job.id.clone();
            self.invalidate_balance_for_job();
            self.busy = true;
            self.error = None;
            self.send(Command::Resume(id));
        }
    }
    pub fn restore(&mut self) {
        if self.busy || self.offline {
            return;
        }
        let id = self.restore_id.text.trim().to_string();
        if id.is_empty() {
            self.error = Some("invalid_task_id".into());
            return;
        }
        self.invalidate_balance_for_job();
        self.busy = true;
        self.error = None;
        self.result = None;
        self.job = None;
        self.send(Command::Restore(id));
    }
    pub fn load_job(&mut self, id: String) {
        if self.busy || self.offline {
            return;
        }
        self.page = Page::Create;
        self.result = None;
        self.error = None;
        self.job = self.history.iter().find(|j| j.id == id).cloned();
        if self.job.as_ref().is_some_and(|j| j.image_path.is_some()) {
            self.busy = true;
            self.send(Command::Load(id));
        }
    }
    pub fn export(&mut self, path: PathBuf) {
        if self.busy || self.offline {
            return;
        }
        if let Some(job) = &self.job {
            if job.image_path.is_some() {
                self.send(Command::Export(job.id.clone(), path));
            }
        }
    }
    fn photo(&mut self, path: PathBuf) -> Photo {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if !self.offline && self.sender.is_some() {
            self.send(Command::Thumbnail(path.clone()));
        }
        Photo {
            path,
            name,
            image: None,
        }
    }
    pub fn add_photo(&mut self, path: PathBuf) {
        if self.busy {
            return;
        }
        if self.photos.len() >= 8 {
            self.error = Some("too_many_photos".into());
            return;
        }
        if !valid_photo_extension(&path) {
            self.error = Some("unsupported_image".into());
            return;
        }
        if self.photos.iter().any(|p| p.path == path) {
            return;
        }
        let photo = self.photo(path);
        self.photos.push(photo);
        self.error = None;
        self.request_quote();
    }
    pub fn remove_photo(&mut self, index: usize) {
        if !self.busy && index < self.photos.len() {
            self.photos.remove(index);
            self.request_quote();
        }
    }
    pub fn set_reference(&mut self, path: PathBuf) {
        if self.busy {
            return;
        }
        if !valid_photo_extension(&path) {
            self.error = Some("unsupported_image".into());
            return;
        }
        self.custom_reference = Some(self.photo(path));
        self.error = None;
        self.request_quote();
    }
    pub fn clear_reference(&mut self) {
        if !self.busy {
            self.custom_reference = None;
            self.request_quote();
        }
    }
    pub fn poll(&mut self, _dt: f64, _tx: &mut Tx) -> bool {
        let mut changed = false;
        while let Some(event) = self.events.as_ref().and_then(|r| r.try_recv().ok()) {
            changed = true;
            match event {
                Event::Boot(Ok((lang, history))) => {
                    self.lang = lang;
                    self.history = history;
                    self.booted = true;
                    self.request_quote();
                    self.refresh_balance();
                }
                Event::Boot(Err(error)) => {
                    self.booted = true;
                    self.error = Some(error);
                }
                Event::Quote(id, result) if id == self.quote_id => {
                    self.quote_pending = false;
                    match result {
                        Ok(cost) => {
                            self.quote = Some(cost);
                            if self.job.is_none()
                                && self.error.as_deref().is_some_and(|code| {
                                    matches!(
                                        code,
                                        "network"
                                            | "network_error"
                                            | "missing_key"
                                            | "auth"
                                            | "scope"
                                            | "rate_limit"
                                            | "maintenance"
                                            | "unknown_price"
                                    )
                                })
                            {
                                self.error = None;
                            }
                        }
                        Err(error) => {
                            self.quote = None;
                            self.error = Some(error);
                        }
                    }
                }
                Event::Quote(..) => (),
                Event::Balance(id, result)
                    if id == self.balance_generation.load(Ordering::Relaxed) =>
                {
                    self.balance_pending = false;
                    match result {
                        Ok(balance) => {
                            self.balance = Some(balance);
                            self.balance_error = None;
                        }
                        Err(error) => {
                            self.balance = None;
                            self.balance_error = Some(error);
                        }
                    }
                }
                Event::Balance(..) => (),
                Event::Progress(job) => {
                    self.job = Some(job);
                }
                Event::Finished(result) => {
                    self.busy = false;
                    match result {
                        Ok((job, bytes)) => {
                            self.job = Some(job);
                            self.result = self.cache.as_ref().map(|c| c.load(bytes));
                            self.error = None;
                        }
                        Err(error) => {
                            if matches!(error.as_str(), "price_changed" | "unknown_price") {
                                self.quote = None;
                            }
                            self.error = Some(error);
                        }
                    }
                    if self.quote.is_none() {
                        self.request_quote();
                    }
                    if self.balance_after_job {
                        self.balance_after_job = false;
                        self.refresh_balance();
                    }
                }
                Event::History(jobs) => self.history = jobs,
                Event::Exported(Ok(())) => self.notice = Some("saved".into()),
                Event::Exported(Err(error)) => self.error = Some(error),
                Event::Thumbnail(path, Ok(bytes)) => {
                    if let Some(cache) = &self.cache {
                        let image = cache.load(bytes);
                        for photo in &mut self.photos {
                            if photo.path == path {
                                photo.image = Some(image.clone());
                            }
                        }
                        if let Some(photo) = &mut self.custom_reference {
                            if photo.path == path {
                                photo.image = Some(image);
                            }
                        }
                    }
                }
                Event::Thumbnail(_, Err(error)) => self.error = Some(error),
            }
        }
        changed
    }
}
fn valid_photo_extension(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp"
        )
    })
}
fn worker(
    commands: Receiver<Command>,
    balance_requests: Receiver<u64>,
    events: Sender<Event>,
    wake: Waker,
    quote_generation: Arc<AtomicU64>,
    balance_generation: Arc<AtomicU64>,
) {
    let emit = |event| {
        let _ = events.send(event);
        wake();
    };
    let Some(base) = directories::BaseDirs::new() else {
        emit(Event::Boot(Err("storage_error".into())));
        return;
    };
    let data_dir = base.data_dir().join("app.chiikawizator.studio");
    let preference_path = data_dir.join("rune-language.json");
    let engine = match Engine::new(data_dir) {
        Ok(engine) => engine,
        Err(error) => {
            emit(Event::Boot(Err(error)));
            return;
        }
    };
    // Account reads have their own bounded, serial queue, so a slow balance
    // request cannot hold up uploading photos, rendering, or saving the original.
    let balance_engine = engine.clone();
    let balance_events = events.clone();
    let balance_wake = wake.clone();
    std::thread::Builder::new()
        .name("drawing-balance".into())
        .spawn(move || {
            for id in balance_requests {
                if id != balance_generation.load(Ordering::Relaxed) {
                    continue;
                }
                let result = balance_engine.balance();
                let _ = balance_events.send(Event::Balance(id, result));
                balance_wake();
            }
        })
        .expect("start balance worker");
    let language = std::fs::read(&preference_path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Lang>(&b).ok())
        .unwrap_or(Lang::Ru);
    match engine.list() {
        Ok(jobs) => emit(Event::Boot(Ok((language, jobs)))),
        Err(error) => {
            emit(Event::Boot(Err(error)));
            return;
        }
    }
    for command in commands {
        match command {
            Command::Quote(id, request) => {
                if id == quote_generation.load(Ordering::Relaxed) {
                    emit(Event::Quote(id, engine.quote(&request).map(|q| q.credits)));
                }
            }
            Command::Language(lang) => {
                if let Ok(bytes) = serde_json::to_vec(&lang) {
                    let _ = std::fs::write(&preference_path, bytes);
                }
            }
            Command::Export(id, path) => emit(Event::Exported(engine.export(&id, &path))),
            Command::Thumbnail(path) => {
                let bytes = thumbnail(&path);
                emit(Event::Thumbnail(path, bytes));
            }
            command => {
                let progress = |job| emit(Event::Progress(job));
                let result = match command {
                    Command::Generate(request) => engine.generate(&request, progress),
                    Command::Resume(id) => engine.resume(&id, progress),
                    Command::Restore(id) => engine.restore(&id, progress),
                    Command::Load(id) => engine.list().and_then(|jobs| {
                        jobs.into_iter()
                            .find(|j| j.id == id)
                            .ok_or_else(|| "task_not_found".into())
                    }),
                    _ => unreachable!(),
                }
                .and_then(|job| engine.load_image(&job.id).map(|bytes| (job, bytes)));
                emit(Event::Finished(result));
                if let Ok(history) = engine.list() {
                    emit(Event::History(history));
                }
            }
        }
    }
}
fn thumbnail(path: &std::path::Path) -> Result<Vec<u8>, String> {
    backend::thumbnail(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_photo_and_eight_file_limit_are_local() {
        let mut state = State::new();
        state.offline = true;
        state.add_photo("a.txt".into());
        assert!(state.photos.is_empty());
        for i in 0..9 {
            state.add_photo(format!("photo-{i}.jpg").into());
        }
        assert_eq!(state.photos.len(), 8);
        assert_eq!(state.error.as_deref(), Some("too_many_photos"));
        state.remove_photo(2);
        assert_eq!(state.photos.len(), 7);
    }
    #[test]
    fn default_mode_preserves_classic_transformation() {
        let state = State::new();
        assert!(!state.request.full_creature);
        assert_eq!(state.lang, Lang::Ru);
    }
    #[test]
    fn offline_mode_never_starts_paid_generation() {
        let mut state = State::new();
        state.offline = true;
        state.quote = Some(3.);
        state.generate();
        assert!(!state.busy);
        assert!(state.job.is_none());
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "state_balance_tests.rs"]
mod balance_tests;
