#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod backend;
mod i18n;
mod state;
mod ui;
use i18n::Lang;
use rune::{App, AppSpec};
use state::State;
const LATIN: &[u8] = include_bytes!("../assets/fonts/ChiikawaKawaii-Regular.ttf");
const JAPANESE: &[u8] = include_bytes!("../assets/fonts/ChiikawaKawaiiJP-Regular.ttf");
const FAMILY: &str = "Chiikawa Kawaii, Chiikawa Kawaii JP";
fn spec() -> AppSpec<State> {
    AppSpec {
        min_size: Some((1100., 720.)),
        fonts: &[LATIN, JAPANESE],
        families: (FAMILY, FAMILY),
        setup: Some(State::setup),
        poll: Some(State::poll),
        scenes: &[
            ("home-ru", |s| s.lang = Lang::Ru),
            ("home-en", |s| s.lang = Lang::En),
            ("home-ja", |s| s.lang = Lang::Ja),
            ("photos-ru", |s| {
                s.lang = Lang::Ru;
                s.request.mode = "photo".into();
            }),
            ("history-en", |s| {
                s.lang = Lang::En;
                s.page = state::Page::History;
            }),
            ("settings-ja", |s| {
                s.lang = Lang::Ja;
                s.page = state::Page::Settings;
            }),
        ],
        ..AppSpec::new("Chiikawizator", (1280., 850.), State::new, ui::view)
    }
}
fn main() {
    if std::env::args().any(|arg| arg == "--self-test") {
        let characters = &backend::CHARACTER_IDS[2..];
        let refs_ok = characters.iter().all(|id| {
            backend::reference_bytes(id).is_some_and(|b| image::load_from_memory(b).is_ok())
        });
        let ok = refs_ok && LATIN.len() > 50_000 && JAPANESE.len() > 1_000_000;
        println!(
            "{}",
            serde_json::json!({"ok":ok,"version":env!("CARGO_PKG_VERSION"),"renderer":"Rune","characters":characters,"hasEmbeddedKey":backend::Engine::has_embedded_key(),"languages":["ru","en","ja"]})
        );
        if !ok {
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--check-connection") {
        let probe = tempfile::tempdir();
        let result = probe
            .as_ref()
            .map_err(|_| "storage_error".to_string())
            .and_then(|dir| backend::Engine::new(dir.path().to_path_buf()))
            .and_then(|engine| {
                let quote = engine.quote(&backend::GenerateRequest::default())?;
                let balance = engine.balance()?;
                Ok((quote, balance))
            });
        match result {
            Ok((quote, balance)) => println!(
                "{}",
                serde_json::json!({"ok":true,"credits":quote.credits,"balance":balance})
            ),
            Err(code) => {
                println!("{}", serde_json::json!({"ok":false,"error":code}));
                std::process::exit(1);
            }
        }
        return;
    }
    let native_smoke = std::env::var_os("CHIIKAWIZATOR_OFFLINE").is_some()
        && std::env::args().any(|arg| arg == "--native");
    let mut app = App::new(spec()).tab_visits_controls(true);
    let handle = app.handle();
    if native_smoke {
        // A directly spawned bundle executable does not receive Launch Services'
        // foreground activation. Count real, visible frames in CI as in a user launch.
        handle.open();
        app = app.on_frame_encoded(|frame| {
            if frame.seq() < 20 {
                eprintln!("native-smoke frame {} {:?}", frame.seq(), frame.outcome());
            }
        });
    }
    app.on_ready(move |_| {
        if native_smoke {
            handle.focus();
        }
        handle.update(|state, _| state.start());
    })
    .run();
}
