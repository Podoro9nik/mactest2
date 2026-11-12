use super::*;
use rune::{Env, FileDialogKind, Runtime, text::TextSystem};
use std::{path::PathBuf, sync::Arc};

fn app() -> Runtime<State> {
    app_at((1280.0, 850.0))
}
fn app_at(size: (f32, f32)) -> Runtime<State> {
    render(app_state(), size)
}
fn app_state() -> State {
    let mut state = State::new();
    state.offline = true;
    state.setup(Arc::new(|| {}));
    for image in state.assets.values() {
        image.wait().expect("fixture image");
    }
    state
}
fn render(state: State, size: (f32, f32)) -> Runtime<State> {
    let fonts = TextSystem::new(
        &[crate::LATIN, crate::JAPANESE],
        (crate::FAMILY, crate::FAMILY),
    );
    let mut runtime = Runtime::with_text(state, view, size, Env::new(false, 1.0), fonts);
    runtime.frame(0.0);
    runtime
}
fn tap(runtime: &mut Runtime<State>, key: &str) {
    runtime.update(|_, tx| {
        tx.scroll_into_view(key);
        tx.instant();
    });
    runtime.frame(0.0);
    for _ in 0..20 {
        runtime.frame(0.05);
    }
    let (x, y) = runtime.find(key).unwrap_or_else(|| panic!("missing {key}"));
    assert!(
        (0.0..1280.0).contains(&x) && (0.0..850.0).contains(&y),
        "{key} is outside the window: {x},{y}"
    );
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/characters")
        .join(name)
}

#[test]
fn inspiration_and_languages_are_clickable_without_starting_a_job() {
    let mut runtime = app();
    tap(&mut runtime, "idea-picnic");
    assert!(runtime.state.description.text.contains("пикник"));
    tap(&mut runtime, "language-en");
    tap(&mut runtime, "idea-stars");
    assert!(runtime.state.description.text.contains("shooting star"));
    tap(&mut runtime, "language-ja");
    tap(&mut runtime, "idea-bakery");
    assert!(runtime.state.description.text.contains("パン屋"));
    assert!(runtime.state.job.is_none());
    assert!(!runtime.state.busy);
}

#[test]
fn photo_dialog_drop_and_remove_preserve_multiple_inputs() {
    let mut runtime = app();
    tap(&mut runtime, "mode-photo");
    assert_eq!(runtime.state.request.mode, "photo");
    tap(&mut runtime, "add-photo");
    assert_eq!(
        runtime.file_dialog().expect("open photo dialog").0,
        FileDialogKind::Open
    );
    runtime
        .resolve_file_dialog(Some(fixture("chiikawa.png")))
        .unwrap();
    runtime.frame(0.0);
    let at = runtime.find("photo-drop");
    runtime.drop_files(vec![fixture("momonga.webp"), fixture("rakko.webp")], at);
    runtime.frame(0.0);
    assert_eq!(runtime.state.photos.len(), 3);
    tap(&mut runtime, "remove-photo-1");
    assert_eq!(runtime.state.photos.len(), 2);
    assert_eq!(runtime.state.photos[1].name, "rakko.webp");
    tap(&mut runtime, "transform-creature");
    assert!(runtime.state.request.full_creature);
    tap(&mut runtime, "mode-text");
    tap(&mut runtime, "mode-photo");
    assert_eq!(runtime.state.photos.len(), 2);
}

#[test]
fn characters_options_and_reference_dialog_bind_to_the_request() {
    let mut runtime = app();
    for id in crate::backend::CHARACTER_IDS {
        tap(&mut runtime, &format!("character-{id}"));
        assert_eq!(&runtime.state.request.character, id);
    }
    tap(&mut runtime, "background-meadow");
    tap(&mut runtime, "emotion-shy");
    tap(&mut runtime, "ratio-16-9");
    tap(&mut runtime, "resolution-4K");
    assert_eq!(runtime.state.request.background, "meadow");
    assert_eq!(runtime.state.request.emotion, "shy");
    assert_eq!(runtime.state.request.aspect_ratio, "16:9");
    assert_eq!(runtime.state.request.resolution, "4K");
    tap(&mut runtime, "reference-add");
    runtime
        .resolve_file_dialog(Some(fixture("shisa.webp")))
        .unwrap();
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.custom_reference.as_ref().unwrap().name,
        "shisa.webp"
    );
    tap(&mut runtime, "reference-remove");
    assert!(runtime.state.custom_reference.is_none());
}

#[test]
fn navigation_survives_language_changes_and_busy_controls_stay_frozen() {
    let mut runtime = app();
    tap(&mut runtime, "nav-settings");
    assert!(runtime.state.page == Page::Settings);
    tap(&mut runtime, "settings-language-ja");
    tap(&mut runtime, "nav-history");
    assert!(runtime.state.page == Page::History);
    tap(&mut runtime, "history-create");
    runtime.update(|s, _| s.busy = true);
    runtime.frame(0.0);
    tap(&mut runtime, "mode-photo");
    assert_eq!(runtime.state.request.mode, "text");
    tap(&mut runtime, "language-en");
    assert_eq!(runtime.state.lang, Lang::En);
    tap(&mut runtime, "nav-settings");
    assert!(runtime.state.page == Page::Settings);
    assert!(runtime.state.busy);
}

#[test]
fn narrow_windows_keep_the_price_and_generate_action_on_screen() {
    for size in [(1100.0, 720.0), (1200.0, 800.0)] {
        let mut runtime = app_at(size);
        for lang in [Lang::Ru, Lang::En, Lang::Ja] {
            runtime.update(|state, _| {
                state.lang = lang;
                state.error = Some("keep_uncertain".into());
                state.balance_error = Some("network_error".into());
            });
            runtime.frame(0.0);
            for key in [
                "sidebar",
                "result-panel",
                "price",
                "balance-panel",
                "balance-refresh",
                "generate",
                "language-ja",
            ] {
                let (rect, _) = runtime.presented(key).unwrap();
                assert!(rect[0] >= 0.0 && rect[1] >= 0.0, "{key}: {rect:?}");
                assert!(
                    rect[0] + rect[2] <= size.0 + 0.5,
                    "{key} overflows horizontally: {rect:?}"
                );
                assert!(
                    rect[1] + rect[3] <= size.1 + 0.5,
                    "{key} overflows vertically: {rect:?}"
                );
            }
            let (form, _) = runtime.presented("form-scroll").unwrap();
            let (result, _) = runtime.presented("result-panel").unwrap();
            assert!(form[0] + form[2] <= result[0], "form and result overlap");
        }
    }
}

#[test]
fn completed_original_is_painted_and_export_opens_a_save_dialog() {
    let mut state = app_state();
    state.result = state.assets.get("chiikawa").cloned();
    assert!(state.result.as_ref().unwrap().is_ready());
    // No other artwork remains in this fixture: an image-atlas page must come
    // from the completed original itself, not from a character-card thumbnail.
    state.assets.clear();
    state.job = Some(crate::backend::Job {
        id: "fixture-completed".into(),
        description: "A completed drawing".into(),
        created_at: "2026-11-13T00:00:00Z".into(),
        stage: crate::backend::Stage::Completed,
        remote_id: Some("fixture-original".into()),
        cost: Some(9.0),
        image_path: Some(fixture("chiikawa.png")),
        error: None,
    });
    let mut runtime = render(state, (1280.0, 850.0));
    let image = runtime
        .elements()
        .into_iter()
        .find(|node| node.key.as_deref() == Some("result-image"))
        .unwrap();
    assert!(image.visible[2] > 100.0 && image.visible[3] > 100.0);
    assert!(
        runtime
            .images
            .pages
            .iter()
            .any(|page| !page.data.is_empty()),
        "completed original has no uploaded pixels"
    );
    assert!(runtime.find("result-placeholder").is_none());
    assert!(runtime.find("resume").is_none());
    tap(&mut runtime, "export");
    let (kind, options) = runtime.file_dialog().expect("save dialog");
    assert_eq!(kind, FileDialogKind::Save);
    assert_eq!(options.file_name.as_deref(), Some("chiikawa.png"));
    runtime.resolve_file_dialog(None).unwrap();
    runtime.frame(0.0);
    assert!(runtime.state.result.is_some());
    runtime.update(|state, _| state.result = None);
    runtime.frame(0.0);
    assert!(
        runtime.find("resume").is_some(),
        "completed jobs with a missing local image must be recoverable"
    );
}

fn shown_text(runtime: &Runtime<State>, key: &str) -> String {
    runtime
        .elements()
        .into_iter()
        .find(|node| node.key.as_deref() == Some(key))
        .and_then(|node| node.text)
        .unwrap_or_else(|| panic!("missing text for {key}"))
}

#[test]
fn remaining_credits_distinguish_zero_unknown_loading_and_failed_refresh() {
    let mut runtime = app();
    for lang in [Lang::Ru, Lang::En, Lang::Ja] {
        runtime.update(|state, _| {
            state.lang = lang;
            state.balance = Some(124.5);
            state.balance_pending = false;
            state.balance_error = None;
        });
        runtime.frame(0.0);
        assert_eq!(shown_text(&runtime, "balance-label"), lang.text("balance"));
        assert_eq!(
            shown_text(&runtime, "balance-value"),
            format!("124.50 {}", lang.text("credits"))
        );
        runtime.update(|state, _| state.balance = Some(0.0));
        runtime.frame(0.0);
        assert_eq!(
            shown_text(&runtime, "balance-value"),
            format!("0.00 {}", lang.text("credits"))
        );
        runtime.update(|state, _| state.balance = None);
        runtime.frame(0.0);
        assert_eq!(
            shown_text(&runtime, "balance-value"),
            lang.text("balance_unknown")
        );
        runtime.update(|state, _| {
            state.balance = Some(124.5);
            state.balance_pending = true;
        });
        runtime.frame(0.0);
        assert_eq!(
            shown_text(&runtime, "balance-value"),
            lang.text("balance_loading")
        );
        tap(&mut runtime, "balance-refresh");
        assert!(
            runtime.state.balance_pending,
            "an in-flight refresh must not restart"
        );
        runtime.update(|state, _| {
            state.balance_pending = false;
            state.balance_error = Some("network_error".into());
        });
        runtime.frame(0.0);
        assert_eq!(
            shown_text(&runtime, "balance-value"),
            lang.text("balance_error"),
            "a stale number must not look current"
        );
        runtime.update(|state, _| state.busy = true);
        runtime.frame(0.0);
        tap(&mut runtime, "balance-refresh");
        assert!(
            runtime.state.balance_error.is_some(),
            "refresh must stay disabled during generation"
        );
        runtime.update(|state, _| state.busy = false);
        runtime.frame(0.0);
        tap(&mut runtime, "balance-refresh");
        assert!(runtime.state.balance_error.is_none());
        assert!(runtime.state.balance.is_some());
        assert!(!runtime.state.balance_pending);
    }
}
