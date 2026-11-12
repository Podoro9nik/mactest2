use super::*;
use crate::backend::Stage;
use std::{io::Cursor, sync::mpsc::TryRecvError};

/// No worker is started: these tests inspect exactly which commands a UI
/// transition would send, without opening storage, making requests, or spending.
fn harness() -> (State, Sender<Event>, Receiver<Command>) {
    let (commands_tx, commands_rx) = mpsc::channel();
    let (events_tx, events_rx) = mpsc::channel();
    let mut state = State::new();
    state.offline = false;
    state.booted = true;
    state.sender = Some(commands_tx);
    state.events = Some(events_rx);
    state.description.replace_text("Friends have a picnic.");
    (state, events_tx, commands_rx)
}

fn poll(state: &mut State) {
    assert!(state.poll(0.016, &mut Tx::default()));
    assert!(!state.poll(0.016, &mut Tx::default()));
}

fn job(stage: Stage) -> Job {
    Job {
        id: "r_00000000-0000-4000-8000-000000000001".into(),
        description: "A saved drawing".into(),
        created_at: "2026-11-13T00:00:00Z".into(),
        stage,
        remote_id: Some("g_existing".into()),
        cost: Some(9.0),
        image_path: None,
        error: None,
    }
}

fn quote_command(commands: &Receiver<Command>) -> (u64, GenerateRequest) {
    match commands.try_recv().expect("expected a price request") {
        Command::Quote(id, request) => (id, request),
        _ => panic!("a transition sent a mutation instead of a read-only price request"),
    }
}

fn no_commands(commands: &Receiver<Command>) {
    assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
}

#[test]
fn stale_price_responses_cannot_enable_generation_or_replace_current_price() {
    let (mut state, events, commands) = harness();
    state.request_quote();
    let (old, _) = quote_command(&commands);
    state.request.resolution = "4K".into();
    state.request_quote();
    let (current, request) = quote_command(&commands);
    assert!(current > old);
    assert_eq!(request.resolution, "4K");
    assert_eq!(state.quote_generation.load(Ordering::Relaxed), current);

    events.send(Event::Quote(old, Ok(1.0))).unwrap();
    poll(&mut state);
    assert_eq!(state.quote, None);
    assert!(state.quote_pending);
    no_commands(&commands);

    events.send(Event::Quote(current, Ok(18.0))).unwrap();
    events.send(Event::Quote(old, Ok(1.0))).unwrap();
    events
        .send(Event::Quote(old, Err("network".into())))
        .unwrap();
    poll(&mut state);
    assert_eq!(state.quote, Some(18.0));
    assert!(!state.quote_pending);
    assert_eq!(state.error, None);
    assert!(!state.busy);
    no_commands(&commands);
}

#[test]
fn fresh_quote_preserves_actionable_payment_and_image_errors() {
    for code in ["keep_uncertain", "image_invalid", "download_failed"] {
        let (mut state, events, commands) = harness();
        state.error = Some(code.into());
        state.request_quote();
        let (id, _) = quote_command(&commands);
        events.send(Event::Quote(id, Ok(9.0))).unwrap();
        poll(&mut state);
        assert_eq!(state.quote, Some(9.0));
        assert_eq!(state.error.as_deref(), Some(code));
        no_commands(&commands);
    }
}

#[test]
fn changed_or_missing_price_requests_a_new_quote_without_restarting_generation() {
    for code in ["price_changed", "unknown_price"] {
        let (mut state, events, commands) = harness();
        state.quote = Some(9.0);
        state.quote_id = 8;
        state.busy = true;
        state.job = Some(job(Stage::Failed));
        events.send(Event::Finished(Err(code.into()))).unwrap();
        poll(&mut state);

        assert!(!state.busy);
        assert!(state.quote_pending);
        assert_eq!(state.quote, None);
        assert_eq!(state.error.as_deref(), Some(code));
        assert_eq!(
            state.job.as_ref().unwrap().remote_id.as_deref(),
            Some("g_existing")
        );
        let (id, request) = quote_command(&commands);
        assert_eq!(id, 9);
        assert_eq!(request.max_cost, None);
        no_commands(&commands);

        events.send(Event::Quote(8, Ok(9.0))).unwrap();
        poll(&mut state);
        assert_eq!(state.quote, None);
        assert!(state.quote_pending);
        events.send(Event::Quote(id, Ok(12.0))).unwrap();
        poll(&mut state);
        assert_eq!(state.quote, Some(12.0));
        assert!(!state.busy);
        no_commands(&commands);
    }
}

#[test]
fn uncertain_payment_and_download_failures_never_send_generate_or_resume() {
    for code in [
        "keep_uncertain",
        "download_failed",
        "network",
        "insufficient_credits",
    ] {
        for old_quote in [None, Some(9.0)] {
            let (mut state, events, commands) = harness();
            state.quote = old_quote;
            state.busy = true;
            let mut failed = job(Stage::Failed);
            failed.error = Some(code.into());
            events.send(Event::Progress(failed)).unwrap();
            events.send(Event::Finished(Err(code.into()))).unwrap();
            poll(&mut state);

            assert!(!state.busy);
            assert_eq!(state.error.as_deref(), Some(code));
            assert_eq!(state.job.as_ref().unwrap().error.as_deref(), Some(code));
            assert!(state.result.is_none());
            if old_quote.is_none() {
                quote_command(&commands);
            }
            no_commands(&commands);
        }
    }
}

#[test]
fn refreshing_a_saved_job_requires_an_explicit_click_and_sends_only_its_id() {
    let (mut state, events, commands) = harness();
    state.quote = Some(9.0);
    let mut saved = job(Stage::Failed);
    saved.error = Some("keep_uncertain".into());
    events.send(Event::History(vec![saved.clone()])).unwrap();
    poll(&mut state);
    state.load_job(saved.id.clone());
    assert!(!state.busy);
    no_commands(&commands);

    state.resume();
    assert!(state.busy);
    match commands.try_recv().unwrap() {
        Command::Resume(id) => assert_eq!(id, saved.id),
        _ => panic!("explicit refresh must send Resume, never Generate"),
    }
    state.resume();
    no_commands(&commands);
}

#[test]
fn generate_captures_displayed_price_and_double_click_cannot_create_two_jobs() {
    let (mut state, _, commands) = harness();
    state.quote = Some(9.0);
    state.generate();
    assert!(state.busy);
    match commands.try_recv().unwrap() {
        Command::Generate(request) => {
            assert_eq!(request.max_cost, Some(9.0));
            assert_eq!(request.description, "Friends have a picnic.");
        }
        _ => panic!("an explicit Generate click must capture the displayed price"),
    }
    state.generate();
    no_commands(&commands);
}

#[test]
fn generation_without_a_known_price_only_requests_a_quote() {
    let (mut state, _, commands) = harness();
    state.generate();
    assert!(!state.busy);
    assert!(state.quote_pending);
    assert_eq!(state.error.as_deref(), Some("unknown_price"));
    assert_eq!(quote_command(&commands).1.max_cost, None);
    no_commands(&commands);
}

#[test]
fn loading_history_at_boot_does_not_resume_locked_or_uncertain_jobs() {
    let (mut state, events, commands) = harness();
    state.booted = false;
    let mut saved = job(Stage::Failed);
    saved.error = Some("legacy_locked".into());
    events
        .send(Event::Boot(Ok((Lang::Ja, vec![saved]))))
        .unwrap();
    poll(&mut state);
    assert!(state.booted);
    assert_eq!(state.lang, Lang::Ja);
    assert_eq!(state.history.len(), 1);
    assert!(!state.busy);
    assert!(state.job.is_none());
    quote_command(&commands);
    no_commands(&commands);
}

#[test]
fn thumbnails_fit_a_small_png_and_preserve_aspect_ratio() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wide.png");
    image::RgbImage::from_pixel(960, 480, image::Rgb([80, 150, 210]))
        .save(&path)
        .unwrap();
    let bytes = thumbnail(&path).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let image = image::load_from_memory(&bytes).unwrap();
    assert!(image.width() <= 240 && image.height() <= 240);
    assert_eq!(image.width(), image.height() * 2);
    assert!(bytes.len() < 128 * 1024);
}

#[test]
fn small_thumbnails_are_not_upscaled() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("small.png");
    image::RgbImage::from_pixel(48, 32, image::Rgb([80, 150, 210]))
        .save(&path)
        .unwrap();
    let image = image::load_from_memory(&thumbnail(&path).unwrap()).unwrap();
    assert_eq!((image.width(), image.height()), (48, 32));
}

#[test]
fn portrait_jpeg_exif_orientation_is_applied_to_its_thumbnail() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rotated.jpg");
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        80,
        40,
        image::Rgb([80, 150, 210]),
    ))
    .write_to(&mut encoded, image::ImageFormat::Jpeg)
    .unwrap();
    let jpeg = encoded.into_inner();
    // Minimal little-endian TIFF with orientation=6 (90 degrees clockwise),
    // carried by an EXIF APP1 segment immediately after the JPEG SOI marker.
    let exif: &[u8] = &[
        b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1, 0,
        0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut oriented = jpeg[..2].to_vec();
    oriented.extend([0xff, 0xe1]);
    oriented.extend(((exif.len() + 2) as u16).to_be_bytes());
    oriented.extend(exif);
    oriented.extend(&jpeg[2..]);
    std::fs::write(&path, oriented).unwrap();
    let image = image::load_from_memory(&thumbnail(&path).unwrap()).unwrap();
    assert_eq!((image.width(), image.height()), (40, 80));
}

#[test]
fn oversized_or_invalid_thumbnail_input_is_rejected_without_creating_an_image() {
    let directory = tempfile::tempdir().unwrap();
    let huge = directory.path().join("oversized.jpg");
    std::fs::File::create(&huge)
        .unwrap()
        .set_len(36 * 1024 * 1024)
        .unwrap();
    assert_eq!(thumbnail(&huge).unwrap_err(), "image_too_large");
    let invalid = directory.path().join("invalid.png");
    std::fs::write(&invalid, b"this is not an image").unwrap();
    assert!(thumbnail(&invalid).is_err());
}

#[test]
fn adding_photos_queues_thumbnail_work_and_never_reads_the_file_on_the_ui_thread() {
    let (mut state, _, commands) = harness();
    state.request.mode = "photo".into();
    let path = PathBuf::from("nonexistent-photo-for-worker.png");
    state.add_photo(path.clone());
    assert_eq!(state.photos.len(), 1);
    assert!(state.photos[0].image.is_none());
    assert_eq!(state.error, None);
    match commands.try_recv().unwrap() {
        Command::Thumbnail(sent) => assert_eq!(sent, path),
        _ => panic!("photo decoding must be queued on the worker"),
    }
    quote_command(&commands);
    no_commands(&commands);
}
