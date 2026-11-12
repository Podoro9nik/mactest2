use super::*;
use serde_json::json;

struct Fake {
    task: RemoteTask,
    keeps: usize,
    downloads: usize,
    lose_keep_reply: bool,
    apply_payment: bool,
    fail_download: bool,
}

impl Fake {
    fn new() -> Self {
        Self {
            task: RemoteTask {
                job_id: "g_aabbcc".into(),
                gen_id: Some("g_aabbcc".into()),
                status: "preview".into(),
                url: Some("https://example.invalid/locked".into()),
                cost: Some(6.0),
            },
            keeps: 0,
            downloads: 0,
            lose_keep_reply: false,
            apply_payment: true,
            fail_download: false,
        }
    }
}

impl Provider for Fake {
    fn status(&mut self, _: &RemoteTask) -> AppResult<RemoteTask> {
        Ok(self.task.clone())
    }
    fn keep(&mut self, _: &RemoteTask, _: f64) -> AppResult<RemoteTask> {
        self.keeps += 1;
        if self.apply_payment {
            self.task.status = "completed".into();
            self.task.url = Some("https://example.invalid/original".into());
        }
        if self.lose_keep_reply {
            Err("keep_uncertain".into())
        } else {
            Ok(self.task.clone())
        }
    }
    fn download(&mut self, url: &str) -> AppResult<Vec<u8>> {
        assert_eq!(url, "https://example.invalid/original");
        self.downloads += 1;
        if self.fail_download {
            return Err("download_failed".into());
        }
        imaging::png_bytes(&image::DynamicImage::new_rgb8(256, 256))
    }
}

fn record(fake: &Fake) -> Record {
    Record {
        schema: 4,
        id: format!("r_{}", uuid::Uuid::new_v4()),
        description: "Friends".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        consent: Consent::Automatic { max_cost: 6.0 },
        state: WorkState::Submitted {
            remote: fake.task.clone(),
        },
        last_error: None,
    }
}

#[test]
fn original_is_unlocked_once_and_no_locked_image_reaches_ui() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new(dir.path().into()).unwrap();
    let mut fake = Fake::new();
    let mut job = record(&fake);
    let mut updates = Vec::new();
    let completed = engine
        .run_record(&mut job, &mut fake, &mut |value| updates.push(value))
        .unwrap();
    assert_eq!(completed.stage, Stage::Completed);
    assert!(completed.image_path.as_ref().unwrap().is_file());
    for update in updates {
        if update.stage != Stage::Completed {
            assert!(update.image_path.is_none());
        }
    }
    assert_eq!(fake.keeps, 1);
    assert_eq!(fake.downloads, 1);
    engine.run_record(&mut job, &mut fake, &mut |_| {}).unwrap();
    assert_eq!(fake.keeps, 1);
    assert_eq!(fake.downloads, 1);
}

#[test]
fn paid_request_with_lost_response_recovers_after_restart_without_repaying() {
    let dir = tempfile::tempdir().unwrap();
    let mut fake = Fake::new();
    fake.lose_keep_reply = true;
    let mut job = record(&fake);
    {
        let engine = Engine::new(dir.path().into()).unwrap();
        assert_eq!(
            engine
                .run_record(&mut job, &mut fake, &mut |_| {})
                .unwrap_err(),
            "keep_uncertain"
        );
        assert!(matches!(
            engine.store.read(&job.id).unwrap().state,
            WorkState::Unlocking { .. }
        ));
    }
    let engine = Engine::new(dir.path().into()).unwrap();
    let mut persisted = engine.store.read(&job.id).unwrap();
    assert_eq!(
        engine
            .run_record(&mut persisted, &mut fake, &mut |_| {})
            .unwrap()
            .stage,
        Stage::Completed
    );
    assert_eq!(fake.keeps, 1);
}

#[test]
fn uncertain_but_still_locked_payment_is_never_replayed() {
    let dir = tempfile::tempdir().unwrap();
    let mut fake = Fake::new();
    fake.lose_keep_reply = true;
    fake.apply_payment = false;
    let mut job = record(&fake);
    {
        let engine = Engine::new(dir.path().into()).unwrap();
        assert_eq!(
            engine
                .run_record(&mut job, &mut fake, &mut |_| {})
                .unwrap_err(),
            "keep_uncertain"
        );
    }
    let engine = Engine::new(dir.path().into()).unwrap();
    let mut persisted = engine.store.read(&job.id).unwrap();
    for _ in 0..2 {
        assert_eq!(
            engine
                .run_record(&mut persisted, &mut fake, &mut |_| {})
                .unwrap_err(),
            "keep_uncertain"
        );
    }
    assert_eq!(fake.keeps, 1);
    assert_eq!(fake.downloads, 0);
}

#[test]
fn download_failure_remembers_unlock_before_retrying() {
    let dir = tempfile::tempdir().unwrap();
    let mut fake = Fake::new();
    fake.fail_download = true;
    let mut job = record(&fake);
    {
        let engine = Engine::new(dir.path().into()).unwrap();
        assert_eq!(
            engine
                .run_record(&mut job, &mut fake, &mut |_| {})
                .unwrap_err(),
            "download_failed"
        );
        assert!(matches!(
            engine.store.read(&job.id).unwrap().state,
            WorkState::Unlocked { .. }
        ));
    }
    fake.fail_download = false;
    let engine = Engine::new(dir.path().into()).unwrap();
    let mut persisted = engine.store.read(&job.id).unwrap();
    engine
        .run_record(&mut persisted, &mut fake, &mut |_| {})
        .unwrap();
    assert_eq!(fake.keeps, 1);
    assert_eq!(fake.downloads, 2);
}

#[test]
fn changed_or_unknown_final_price_never_spends() {
    for cost in [Some(9.0), None] {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::new(dir.path().into()).unwrap();
        let mut fake = Fake::new();
        fake.task.cost = cost;
        let mut job = record(&fake);
        assert_eq!(
            engine
                .run_record(&mut job, &mut fake, &mut |_| {})
                .unwrap_err(),
            if cost.is_some() {
                "price_changed"
            } else {
                "unknown_price"
            }
        );
        assert_eq!(fake.keeps, 0);
        assert_eq!(fake.downloads, 0);
    }
}

#[test]
fn old_manual_preview_and_imported_ids_have_no_spending_authority() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("jobs")).unwrap();
    let mut fake = Fake::new();
    let uuid = uuid::Uuid::new_v4();
    let old = json!({"job":{"id":format!("c_{uuid}"),"description":"Legacy","createdAt":"2026-01-01","status":"preview","imagePath":"preview.png"},"remote":fake.task});
    std::fs::write(
        dir.path().join("jobs").join(format!("c_{uuid}.json")),
        old.to_string(),
    )
    .unwrap();
    let engine = Engine::new(dir.path().into()).unwrap();
    let mut job = engine.store.read(&format!("r_{uuid}")).unwrap();
    assert!(job.public().image_path.is_none());
    assert_eq!(
        engine
            .run_record(&mut job, &mut fake, &mut |_| {})
            .unwrap_err(),
        "original_locked"
    );
    assert_eq!(fake.keeps, 0);
    // A later external unlock can be read without a second paid action.
    fake.task.status = "completed".into();
    fake.task.url = Some("https://example.invalid/original".into());
    engine.run_record(&mut job, &mut fake, &mut |_| {}).unwrap();
    assert_eq!(fake.keeps, 0);
}

#[test]
fn store_is_exclusive_and_export_cannot_read_an_unrelated_file() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new(dir.path().into()).unwrap();
    assert!(matches!(Engine::new(dir.path().into()), Err(code) if code == "already_running"));
    let outside = dir.path().join("private.png");
    std::fs::write(&outside, b"private").unwrap();
    assert!(engine.store.image_path(&outside).is_err());
}

#[test]
fn every_character_reference_decodes_and_is_named_in_prompt() {
    prompts::reference_smoke_test().unwrap();
    for name in ["momonga", "rakko", "kurimanju", "shisa", "furuhonya"] {
        let request = GenerateRequest {
            description: "A picnic".into(),
            character: name.into(),
            ..Default::default()
        };
        let (prompt, refs) = prompts::prepare(&request).unwrap();
        assert_eq!(refs.len(), 1);
        assert!(prompt.to_lowercase().contains(name));
        assert!(prompt.contains("Never copy labels"));
    }
}

#[test]
fn eight_photos_and_custom_reference_stay_in_order_under_ten_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("photo.png");
    std::fs::write(
        &file,
        imaging::png_bytes(&image::DynamicImage::new_rgb8(256, 256)).unwrap(),
    )
    .unwrap();
    let request = GenerateRequest {
        mode: "photo".into(),
        description: "Together".into(),
        photos: vec![file.clone(); 8],
        custom_reference: Some(file),
        full_creature: true,
        ..Default::default()
    };
    let (_, count) = prompts::quote_input(&request).unwrap();
    let (prompt, refs) = prompts::prepare(&request).unwrap();
    assert_eq!(count, 10);
    assert_eq!(refs.len(), 10);
    assert!(refs[..8].iter().all(|item| item == &refs[0]));
    assert_eq!(refs[0], refs[9]);
    assert!(prompt.contains("Images 1 through 8"));
    assert!(prompt.contains("Images 9 through 9"));
    assert!(prompt.contains("NON-HUMAN"));
}

#[test]
fn noisy_photo_is_encoded_under_the_full_json_limit() {
    let mut seed = 81u32;
    let image = image::RgbImage::from_fn(1600, 1200, |_, _| {
        image::Rgb(std::array::from_fn(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        }))
    });
    let original = imaging::png_bytes(&image::DynamicImage::ImageRgb8(image)).unwrap();
    assert!(original.len() > 4 * 1024 * 1024);
    let prepared = imaging::prepare_bytes(&original).unwrap();
    assert!(prepared.starts_with("data:image/jpeg;base64,"));
    let (_, encoded) = prepared.split_once(',').unwrap();
    let request = json!({"jsonrpc":"2.0","id":u64::MAX,"method":"tools/call","params":{"name":"elyum_upload","arguments":{"base64":encoded,"mimeType":"image/jpeg","filename":"reference-10.jpg"}}});
    assert!(serde_json::to_vec(&request).unwrap().len() < 4 * 1024 * 1024);
}

#[test]
fn small_references_are_not_needlessly_enlarged_to_2048() {
    let source = imaging::png_bytes(&image::DynamicImage::new_rgb8(256, 320)).unwrap();
    let prepared = imaging::prepare_bytes(&source).unwrap();
    let decoded = imaging::decode(&imaging::data_bytes(&prepared).unwrap()).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (256, 320));
}
