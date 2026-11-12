use super::*;
use crate::backend::Stage;
use std::sync::mpsc::TryRecvError;

/// Separate fake queues exercise the same UI boundaries as the two workers.
/// No worker, account request, storage operation, or generation is started.
fn harness() -> (State, Sender<Event>, Receiver<Command>, Receiver<u64>) {
    let (commands_tx, commands_rx) = mpsc::channel();
    let (balance_tx, balance_rx) = mpsc::channel();
    let (events_tx, events_rx) = mpsc::channel();
    let mut state = State::new();
    state.offline = false;
    state.booted = true;
    state.quote = Some(9.0);
    state.sender = Some(commands_tx);
    state.balance_sender = Some(balance_tx);
    state.events = Some(events_rx);
    state.description.replace_text("Friends have a picnic.");
    (state, events_tx, commands_rx, balance_rx)
}

fn poll(state: &mut State) {
    assert!(state.poll(0.016, &mut Tx::default()));
    assert!(!state.poll(0.016, &mut Tx::default()));
}

fn empty<T>(receiver: &Receiver<T>) {
    assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
}

fn take_generate(commands: &Receiver<Command>) {
    match commands
        .try_recv()
        .expect("explicit Generate should be queued")
    {
        Command::Generate(request) => assert_eq!(request.max_cost, Some(9.0)),
        _ => panic!("unexpected command in the generation queue"),
    }
}

fn completed_job() -> Job {
    Job {
        id: "r_00000000-0000-4000-8000-000000000002".into(),
        description: "Friends have a picnic.".into(),
        created_at: "2026-11-13T00:00:00Z".into(),
        stage: Stage::Completed,
        remote_id: Some("g_finished".into()),
        cost: Some(9.0),
        image_path: Some("outputs/original.png".into()),
        error: None,
    }
}

#[test]
fn manual_balance_refresh_is_read_only_and_deduplicated_until_reply() {
    let (mut state, events, commands, balances) = harness();
    state.refresh_balance();
    let first = balances.try_recv().unwrap();
    assert!(state.balance_pending);
    assert_eq!(state.balance, None);
    assert_eq!(state.balance_generation.load(Ordering::Relaxed), first);
    state.refresh_balance();
    state.refresh_balance();
    empty(&balances);
    empty(&commands);

    events.send(Event::Balance(first, Ok(120.0))).unwrap();
    poll(&mut state);
    assert_eq!(state.balance, Some(120.0));
    assert!(!state.balance_pending);
    assert_eq!(state.balance_error, None);
    state.refresh_balance();
    assert!(balances.try_recv().unwrap() > first);
    empty(&commands);
}

#[test]
fn missing_or_failed_balance_is_unknown_while_an_explicit_zero_is_valid() {
    let (mut state, events, commands, balances) = harness();
    assert_eq!(state.balance, None);
    for code in ["balance_unavailable", "network", "auth"] {
        state.balance = Some(120.0);
        state.refresh_balance();
        let id = balances.try_recv().unwrap();
        events.send(Event::Balance(id, Err(code.into()))).unwrap();
        poll(&mut state);
        assert_eq!(state.balance, None);
        assert_eq!(state.balance_error.as_deref(), Some(code));
        assert!(!state.balance_pending);
        empty(&commands);
    }
    state.refresh_balance();
    let id = balances.try_recv().unwrap();
    events.send(Event::Balance(id, Ok(0.0))).unwrap();
    poll(&mut state);
    assert_eq!(state.balance, Some(0.0));
    assert_eq!(state.balance_error, None);

    drop(balances);
    state.refresh_balance();
    assert_eq!(state.balance, None);
    assert!(!state.balance_pending);
    assert_eq!(state.balance_error.as_deref(), Some("balance_unavailable"));
    empty(&commands);
}

#[test]
fn pre_purchase_balance_replies_cannot_replace_the_post_purchase_balance() {
    let (mut state, events, commands, balances) = harness();
    state.refresh_balance();
    let old = balances.try_recv().unwrap();
    state.generate();
    take_generate(&commands);
    assert!(state.balance_after_job);
    assert_eq!(state.balance, None);
    assert!(!state.balance_pending);
    assert!(state.balance_generation.load(Ordering::Relaxed) > old);

    events.send(Event::Balance(old, Ok(120.0))).unwrap();
    events
        .send(Event::Balance(old, Err("network".into())))
        .unwrap();
    poll(&mut state);
    assert!(state.busy);
    assert_eq!(state.balance, None);
    assert_eq!(state.balance_error, None);
    empty(&balances);

    events
        .send(Event::Finished(Ok((completed_job(), Vec::new()))))
        .unwrap();
    poll(&mut state);
    let current = balances.try_recv().unwrap();
    assert!(current > old);
    assert!(state.balance_pending);
    events.send(Event::Balance(old, Ok(120.0))).unwrap();
    poll(&mut state);
    assert_eq!(state.balance, None);
    assert!(state.balance_pending);
    events.send(Event::Balance(current, Ok(111.0))).unwrap();
    events
        .send(Event::Balance(old, Err("auth".into())))
        .unwrap();
    events.send(Event::Balance(old, Ok(120.0))).unwrap();
    poll(&mut state);
    assert_eq!(state.balance, Some(111.0));
    assert_eq!(state.balance_error, None);
    assert!(!state.balance_pending);
    empty(&commands);
}

#[test]
fn finishing_or_uncertain_payment_refreshes_balance_without_retrying_generation() {
    for failure in [None, Some("keep_uncertain"), Some("download_failed")] {
        let (mut state, events, commands, balances) = harness();
        state.generate();
        take_generate(&commands);
        let result = match failure {
            Some(code) => Err(code.into()),
            None => Ok((completed_job(), Vec::new())),
        };
        events.send(Event::Finished(result)).unwrap();
        poll(&mut state);
        assert!(!state.busy);
        assert!(!state.balance_after_job);
        assert!(state.balance_pending);
        assert_eq!(state.error.as_deref(), failure);
        let refresh = balances.try_recv().unwrap();
        empty(&balances);
        empty(&commands);

        events.send(Event::Balance(refresh, Ok(111.0))).unwrap();
        poll(&mut state);
        assert_eq!(state.balance, Some(111.0));
        assert_eq!(state.error.as_deref(), failure);
        empty(&commands);
        empty(&balances);
    }
}

#[test]
fn pending_balance_read_does_not_block_the_separate_generate_queue() {
    let (mut state, _, commands, balances) = harness();
    state.refresh_balance();
    assert!(state.balance_pending);
    // Deliberately do not consume or answer the balance request. Generation
    // must still be delivered immediately on the independent command channel.
    state.generate();
    take_generate(&commands);
    assert!(state.busy);
    state.refresh_balance();
    state.refresh_balance();
    balances.try_recv().unwrap();
    empty(&balances);
    empty(&commands);
}

#[test]
fn offline_balance_is_demo_data_and_never_queues_account_or_generation_requests() {
    let (mut state, _, commands, balances) = harness();
    state.offline = true;
    state.balance = None;
    state.balance_error = Some("network".into());
    state.refresh_balance();
    assert_eq!(state.balance, Some(120.0));
    assert_eq!(state.balance_error, None);
    assert!(!state.balance_pending);
    state.generate();
    assert!(!state.busy);
    empty(&balances);
    empty(&commands);
}
