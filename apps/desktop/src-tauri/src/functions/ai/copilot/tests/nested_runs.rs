use super::*;

#[test]
fn home_run_rejects_a_changed_or_missing_pinned_profile() {
    let context = FrontendToolContext {
        profile_id: Some("profile-a".to_string()),
        ..Default::default()
    };
    assert!(
        home_profile_scope_error(CopilotScope::Home, Some(&context), Some("profile-a")).is_none()
    );
    for selected in [Some("profile-b"), None] {
        let error = home_profile_scope_error(CopilotScope::Home, Some(&context), selected).unwrap();
        let result: serde_json::Value = serde_json::from_str(&error).unwrap();
        assert_eq!(result["status"], "stale");
        assert_eq!(result["code"], "home_profile_changed");
        assert_eq!(result["profile_id"], "profile-a");
    }
    assert!(
        home_profile_scope_error(CopilotScope::Board, Some(&context), Some("profile-b")).is_none()
    );
    assert!(home_profile_scope_error(CopilotScope::Home, None, Some("profile-b")).is_none());
}

#[test]
fn nested_gate_key_prefers_board_then_context_target_then_app() {
    let board = flowscript_recovery_test_board();
    let context = FrontendToolContext {
        board_id: Some("widget-target-board".to_string()),
        ..Default::default()
    };
    assert_eq!(
        nested_copilot_run_gate_key(CopilotScope::Board, Some(&board), Some(&context)),
        format!("board:{}", board.id)
    );
    assert_eq!(
        nested_copilot_run_gate_key(CopilotScope::Board, None, Some(&context)),
        "board:widget-target-board"
    );
    // A board run with no resolved target may create the app's first board, so it serializes
    // per app — but never against a different app.
    let unresolved = FrontendToolContext {
        board_id: Some("   ".to_string()),
        app_id: Some("app-a".to_string()),
        ..Default::default()
    };
    assert_eq!(
        nested_copilot_run_gate_key(CopilotScope::Board, None, Some(&unresolved)),
        "board:unresolved@app-a"
    );
    assert_ne!(
        nested_copilot_run_gate_key(CopilotScope::Board, None, Some(&unresolved)),
        nested_copilot_run_gate_key(
            CopilotScope::Board,
            None,
            Some(&FrontendToolContext {
                app_id: Some("app-b".to_string()),
                ..Default::default()
            })
        )
    );
    assert_eq!(
        nested_copilot_run_gate_key(CopilotScope::Board, None, None),
        "board"
    );
}

#[test]
fn authoring_lanes_do_not_serialize_against_each_other() {
    let board = flowscript_recovery_test_board();
    let context = FrontendToolContext {
        app_id: Some("app-a".to_string()),
        board_id: Some(board.id.clone()),
        ..Default::default()
    };

    // For a mixed user request, app workflow, page, and tables can be built alongside an
    // explicitly requested Home layout because each has a separate mutation lane.
    let board_key = nested_copilot_run_gate_key(CopilotScope::Board, Some(&board), Some(&context));
    let widget_key = nested_copilot_run_gate_key(CopilotScope::Frontend, None, Some(&context));
    let data_key = nested_copilot_run_gate_key(CopilotScope::DataStudio, None, Some(&context));
    let home_key = nested_copilot_run_gate_key(CopilotScope::Home, None, Some(&context));
    assert_eq!(board_key, format!("board:{}", board.id));
    assert_eq!(widget_key, format!("widget:{}", board.id));
    assert_eq!(data_key, "data:app-a");
    assert_eq!(home_key, "home");
    assert_ne!(board_key, widget_key);
    assert_ne!(board_key, data_key);
    assert_ne!(board_key, home_key);
    assert_ne!(widget_key, data_key);
    assert_ne!(widget_key, home_key);
    assert_ne!(data_key, home_key);

    // FrontendToolContext does not expose a profile id, so Home runs share one safe desktop
    // lane even when their app context differs.
    assert_eq!(
        home_key,
        nested_copilot_run_gate_key(
            CopilotScope::Home,
            None,
            Some(&FrontendToolContext {
                app_id: Some("app-b".to_string()),
                ..Default::default()
            })
        )
    );

    // Data work is app-scoped, so two data runs on different apps stay independent while any
    // two overlays in the same app serialize around shared tables and catalogs.
    assert_ne!(
        data_key,
        nested_copilot_run_gate_key(
            CopilotScope::DataStudio,
            None,
            Some(&FrontendToolContext {
                app_id: Some("app-b".to_string()),
                ..Default::default()
            })
        )
    );
    let overlay_context = FrontendToolContext {
        app_id: Some("app-a".to_string()),
        overlay_id: Some("ontology-1".to_string()),
        ..Default::default()
    };
    assert_eq!(
        nested_copilot_run_gate_key(CopilotScope::DataStudio, None, Some(&overlay_context)),
        "data:app-a"
    );
}

#[test]
fn scout_runs_get_per_request_gates_so_candidates_are_researched_in_parallel() {
    let board = flowscript_recovery_test_board();
    let first = FrontendToolContext {
        parent_request_id: Some("req-1".to_string()),
        app_id: Some("app-a".to_string()),
        ..Default::default()
    };
    let second = FrontendToolContext {
        parent_request_id: Some("req-2".to_string()),
        app_id: Some("app-a".to_string()),
        ..Default::default()
    };

    // Two scouts launched from the same turn must not share a gate, even when they are
    // researching the same app — otherwise a parallel fan-out silently runs one at a time.
    assert_eq!(
        nested_copilot_run_gate_key(CopilotScope::Scout, None, Some(&first)),
        "scout:req-1"
    );
    assert_ne!(
        nested_copilot_run_gate_key(CopilotScope::Scout, None, Some(&first)),
        nested_copilot_run_gate_key(CopilotScope::Scout, None, Some(&second))
    );

    // A board in context must not pull a read-only scout onto that board's gate, where it
    // would queue behind an unrelated board edit.
    assert_eq!(
        nested_copilot_run_gate_key(CopilotScope::Scout, Some(&board), Some(&first)),
        "scout:req-1"
    );
    assert_eq!(
        nested_copilot_run_gate_key(CopilotScope::Scout, None, None),
        "scout"
    );
}

#[tokio::test]
async fn nested_gate_serializes_same_board_but_not_different_boards() {
    let gate_a = nested_copilot_run_gate("board:gate-test-a");
    let gate_a_again = nested_copilot_run_gate("board:gate-test-a");
    let gate_b = nested_copilot_run_gate("board:gate-test-b");
    assert!(Arc::ptr_eq(&gate_a, &gate_a_again));

    let owner = gate_a
        .clone()
        .acquire_owned()
        .await
        .expect("first same-board run");
    let cancellation = CancellationToken::new();
    let same_board_waiter = tokio::spawn(acquire_nested_copilot_run_permit(
        gate_a_again,
        cancellation.clone(),
    ));
    let other_board_run = tokio::spawn(acquire_nested_copilot_run_permit(
        gate_b,
        cancellation.clone(),
    ));

    // A different board's run must complete while the same-board run is still queued.
    let other_permit = tokio::time::timeout(Duration::from_secs(1), other_board_run)
        .await
        .expect("a different board must not queue behind this board's run")
        .expect("other-board task")
        .expect("other-board permit");
    tokio::task::yield_now().await;
    assert!(
        !same_board_waiter.is_finished(),
        "a second run on the SAME board must stay queued while the first one runs"
    );

    drop(owner);
    let same_permit = tokio::time::timeout(Duration::from_secs(1), same_board_waiter)
        .await
        .expect("same-board run should acquire promptly after release")
        .expect("same-board task")
        .expect("same-board permit");
    drop(same_permit);
    drop(other_permit);
}

#[tokio::test]
async fn nested_gate_map_prunes_gates_without_holders_or_waiters() {
    let key = "board:gate-prune-test";
    let gate = nested_copilot_run_gate(key);
    let permit = gate
        .clone()
        .acquire_owned()
        .await
        .expect("prune-test permit");
    drop(gate);

    // The held permit keeps an Arc clone alive, so lookups of other keys must not prune it.
    let _other = nested_copilot_run_gate("board:gate-prune-test-other");
    assert!(
        NESTED_COPILOT_RUN_GATES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(key),
        "a gate with an active permit holder must survive pruning"
    );

    drop(permit);
    let _other = nested_copilot_run_gate("board:gate-prune-test-other");
    assert!(
        !NESTED_COPILOT_RUN_GATES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(key),
        "a gate with no holders and no waiters must be pruned on the next lookup"
    );
}

#[tokio::test]
async fn nested_pool_checkout_checkin_and_quarantine_replacement() {
    fn unstarted_client() -> Arc<Client> {
        Arc::new(Client::builder().build().expect("unstarted pool client"))
    }
    // Seed idle clients so checkout never spawns a real CLI process in tests.
    for _ in 0..NESTED_COPILOT_POOL_SIZE {
        let client = unstarted_client();
        assert!(
            NESTED_COPILOT_POOL.register_started(client.clone(), NESTED_COPILOT_POOL.epoch()),
            "seeding an undrained pool must register"
        );
        NESTED_COPILOT_POOL.return_to_idle(client);
    }

    let cancellation = CancellationToken::new();
    // Exhaust exactly the pool, whatever it is sized to, so the cap assertions below stay
    // meaningful when the fan-out width changes.
    let mut leases = Vec::with_capacity(NESTED_COPILOT_POOL_SIZE);
    for slot in 0..NESTED_COPILOT_POOL_SIZE {
        leases.push(
            checkout_nested_copilot_client(cancellation.clone())
                .await
                .unwrap_or_else(|_| panic!("checkout {slot} within the pool cap")),
        );
    }
    for (index, lease) in leases.iter().enumerate() {
        for other in &leases[index + 1..] {
            assert!(
                !Arc::ptr_eq(&lease.client, &other.client),
                "each checked-out lease must exclusively own its own process"
            );
        }
    }
    let mut leases = leases.into_iter();
    let lease_one = leases.next().expect("first checkout");
    let lease_two = leases.next().expect("second checkout");
    let remaining_leases: Vec<_> = leases.collect();

    // All slots busy: a fourth checkout queues, and a cancelled one returns promptly.
    let cancelled_token = CancellationToken::new();
    let cancelled_waiter = tokio::spawn(checkout_nested_copilot_client(cancelled_token.clone()));
    cancelled_token.cancel();
    let cancelled_result = tokio::time::timeout(Duration::from_secs(1), cancelled_waiter)
        .await
        .expect("cancelled checkout should return promptly")
        .expect("cancelled checkout task");
    match cancelled_result {
        Ok(_) => panic!("cancelled checkout must not receive a client"),
        Err(error) => assert!(error.contains("cancelled")),
    }

    let waiter = tokio::spawn(checkout_nested_copilot_client(cancellation.clone()));
    tokio::task::yield_now().await;
    assert!(
        !waiter.is_finished(),
        "a checkout past the pool cap must wait for a checkin"
    );

    // Checkin: dropping a lease returns its exact process to the idle pool.
    let released = lease_one.client();
    drop(lease_one);
    let lease_four = tokio::time::timeout(Duration::from_secs(1), waiter)
        .await
        .expect("queued checkout should acquire promptly after a checkin")
        .expect("queued checkout task")
        .expect("queued checkout lease");
    assert!(
        Arc::ptr_eq(&lease_four.client, &released),
        "the queued checkout must reuse the checked-in idle process"
    );

    // Quarantine: the client leaves the pool, its lease drop must not re-pool it, and the
    // freed slot allows a lazy replacement.
    let quarantined = lease_two.client();
    quarantine_nested_copilot_client(&quarantined).await;
    drop(lease_two);
    assert!(
        !NESTED_COPILOT_POOL.is_registered(&quarantined),
        "a quarantined client must leave the pool registry"
    );
    assert_eq!(
        NESTED_COPILOT_POOL.slots.available_permits(),
        1,
        "the quarantined client's slot must free up for a lazy replacement"
    );

    drop(remaining_leases);
    drop(lease_four);
    let drained = NESTED_COPILOT_POOL.drain();
    assert_eq!(
        drained.len(),
        NESTED_COPILOT_POOL_SIZE - 1,
        "drain must return every live pooled client except the quarantined one"
    );
    assert!(
        drained
            .iter()
            .all(|client| !Arc::ptr_eq(client, &quarantined)),
        "the quarantined client must not reappear in the drained pool"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn nested_gate_stress_serializes_same_board_and_overlaps_across_boards() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const BOARDS: usize = 4;
    const TASKS_PER_BOARD: usize = 8;
    const KEY_PREFIX: &str = "board:stress-gate-";

    let per_board_in_flight: Arc<Vec<AtomicUsize>> =
        Arc::new((0..BOARDS).map(|_| AtomicUsize::new(0)).collect());
    let global_in_flight = Arc::new(AtomicUsize::new(0));
    let max_global_in_flight = Arc::new(AtomicUsize::new(0));
    // One run per board rendezvouses INSIDE its critical section. Per-board gates make this
    // trivially deadlock-free; a global gate would deadlock here and trip the timeout.
    let cross_board_rendezvous = Arc::new(tokio::sync::Barrier::new(BOARDS));

    let mut handles = Vec::new();
    for board in 0..BOARDS {
        for task in 0..TASKS_PER_BOARD {
            let per_board_in_flight = per_board_in_flight.clone();
            let global_in_flight = global_in_flight.clone();
            let max_global_in_flight = max_global_in_flight.clone();
            let cross_board_rendezvous = cross_board_rendezvous.clone();
            handles.push(tokio::spawn(async move {
                let gate = nested_copilot_run_gate(&format!("{KEY_PREFIX}{board}"));
                let permit = acquire_nested_copilot_run_permit(gate, CancellationToken::new())
                    .await
                    .expect("stress gate permit");
                let overlapping = per_board_in_flight[board].fetch_add(1, Ordering::SeqCst);
                assert_eq!(
                    overlapping, 0,
                    "two nested runs overlapped on board {board}"
                );
                let concurrent = global_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                max_global_in_flight.fetch_max(concurrent, Ordering::SeqCst);
                if task == 0 {
                    cross_board_rendezvous.wait().await;
                }
                tokio::time::sleep(Duration::from_millis(((board + task) % 3) as u64)).await;
                global_in_flight.fetch_sub(1, Ordering::SeqCst);
                per_board_in_flight[board].fetch_sub(1, Ordering::SeqCst);
                drop(permit);
            }));
        }
    }
    tokio::time::timeout(Duration::from_secs(8), async {
        for handle in handles {
            handle.await.expect("gate stress task");
        }
    })
    .await
    .expect("per-board gates must never deadlock");

    assert!(
        max_global_in_flight.load(Ordering::SeqCst) >= BOARDS,
        "runs on different boards must overlap; the rendezvous held {BOARDS} boards' gates at once"
    );

    // The map must not leak finished stress gates: any lookup prunes ownerless entries.
    let _probe = nested_copilot_run_gate("board:stress-probe");
    let gates = NESTED_COPILOT_RUN_GATES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(
        gates.keys().all(|key| !key.starts_with(KEY_PREFIX)),
        "finished stress gates must be pruned from the gate map"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn nested_gate_stress_cancelled_waiters_release_and_survivors_proceed() {
    const KEY: &str = "board:stress-gate-cancel";
    let gate = nested_copilot_run_gate(KEY);
    let owner = gate
        .clone()
        .acquire_owned()
        .await
        .expect("initial stress owner");

    let mut cancelled = Vec::new();
    let mut survivors = Vec::new();
    for index in 0..6 {
        let token = CancellationToken::new();
        let waiter_gate = gate.clone();
        let waiter_token = token.clone();
        // Survivors drop their permits inside the task: the semaphore queue order is not the
        // spawn order, so holding permits in unawaited JoinHandles would self-deadlock.
        let handle = tokio::spawn(async move {
            acquire_nested_copilot_run_permit(waiter_gate, waiter_token)
                .await
                .map(drop)
        });
        if index % 2 == 0 {
            cancelled.push((token, handle));
        } else {
            survivors.push(handle);
        }
    }

    // Cancelling queued waiters while the owner still holds the gate must fail them promptly
    // without consuming the permit.
    for (token, handle) in cancelled {
        token.cancel();
        let error = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("cancelled gate waiter must return promptly")
            .expect("cancelled waiter task")
            .expect_err("cancelled waiter must not acquire");
        assert!(error.contains("cancelled"));
    }

    drop(owner);
    // Every surviving waiter must acquire in turn once earlier permits are released.
    tokio::time::timeout(Duration::from_secs(8), async {
        for handle in survivors {
            handle
                .await
                .expect("surviving waiter task")
                .expect("surviving waiter must acquire after cancellations");
        }
    })
    .await
    .expect("cancelled waiters must not strand surviving waiters");

    drop(gate);
    let _probe = nested_copilot_run_gate("board:stress-probe");
    assert!(
        !NESTED_COPILOT_RUN_GATES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(KEY),
        "a fully drained gate must be pruned"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn nested_pool_stress_never_double_leases_and_replaces_quarantined_clients() {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const POOL_SIZE: usize = 3;
    const TASKS: usize = 24;
    let pool = leaked_test_pool(POOL_SIZE);
    let spawned = Arc::new(AtomicUsize::new(0));
    let quarantined = Arc::new(AtomicUsize::new(0));
    let leased: Arc<StdMutex<HashSet<usize>>> = Arc::new(StdMutex::new(HashSet::new()));

    let mut handles = Vec::new();
    for task in 0..TASKS {
        let spawned = spawned.clone();
        let quarantined = quarantined.clone();
        let leased = leased.clone();
        handles.push(tokio::spawn(async move {
            let spawn_counter = spawned.clone();
            let lease = checkout_nested_copilot_client_from(
                pool,
                CancellationToken::new(),
                move || async move {
                    spawn_counter.fetch_add(1, Ordering::SeqCst);
                    Ok(unstarted_pool_client())
                },
            )
            .await
            .expect("stress checkout");
            let key = Arc::as_ptr(&lease.client) as usize;
            assert!(
                leased.lock().expect("lease tracker").insert(key),
                "one pooled client was leased to two concurrent runs"
            );
            tokio::time::sleep(Duration::from_millis((task % 3) as u64)).await;
            if task % 5 == 0 {
                // Quarantine without force_stop: these clients were never started.
                assert!(
                    pool.deregister(&lease.client),
                    "a quarantine target must still be registered"
                );
                quarantined.fetch_add(1, Ordering::SeqCst);
            }
            assert!(leased.lock().expect("lease tracker").remove(&key));
            drop(lease);
        }));
    }
    tokio::time::timeout(Duration::from_secs(8), async {
        for handle in handles {
            handle.await.expect("pool stress task");
        }
    })
    .await
    .expect("more waiters than pool slots must drain without deadlock");

    assert_eq!(
        pool.slots.available_permits(),
        POOL_SIZE,
        "every pool slot must be released after its lease drops"
    );
    let survivors = pool.drain();
    assert!(
        survivors.len() <= POOL_SIZE,
        "the pool must never hold more live clients than slots"
    );
    assert_eq!(
        spawned.load(Ordering::SeqCst),
        survivors.len() + quarantined.load(Ordering::SeqCst),
        "every spawned replacement must end up pooled or quarantined, never lost or duplicated"
    );
}

#[tokio::test]
async fn nested_pool_cancelled_waiters_release_their_queue_positions() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let pool = leaked_test_pool(1);
    let seeded = unstarted_pool_client();
    assert!(pool.register_started(seeded.clone(), pool.epoch()));
    pool.return_to_idle(seeded.clone());

    let spawned = Arc::new(AtomicUsize::new(0));
    let factory = |spawned: Arc<AtomicUsize>| {
        move || async move {
            spawned.fetch_add(1, Ordering::SeqCst);
            Ok(unstarted_pool_client())
        }
    };

    let lease = checkout_nested_copilot_client_from(
        pool,
        CancellationToken::new(),
        factory(spawned.clone()),
    )
    .await
    .expect("initial checkout");

    let mut cancelled = Vec::new();
    let mut survivors = Vec::new();
    for index in 0..4 {
        let token = CancellationToken::new();
        let waiter_token = token.clone();
        let waiter_factory = factory(spawned.clone());
        let expected_client = seeded.clone();
        // Survivors drop their leases inside the task: the slot queue order is not the spawn
        // order, so holding leases in unawaited JoinHandles would self-deadlock the pool.
        let handle = tokio::spawn(async move {
            let lease =
                checkout_nested_copilot_client_from(pool, waiter_token, waiter_factory).await?;
            assert!(
                Arc::ptr_eq(&lease.client, &expected_client),
                "a freed idle client must be reused before any new process spawns"
            );
            drop(lease);
            Ok::<(), String>(())
        });
        if index % 2 == 0 {
            cancelled.push((token, handle));
        } else {
            survivors.push(handle);
        }
    }

    // With the single slot still leased, cancelled waiters must fail promptly and must not
    // consume the slot.
    for (token, handle) in cancelled {
        token.cancel();
        let error = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("cancelled pool waiter must return promptly")
            .expect("cancelled waiter task")
            .expect_err("cancelled waiter must not receive a lease");
        assert!(error.contains("cancelled"));
    }

    drop(lease);
    tokio::time::timeout(Duration::from_secs(8), async {
        for handle in survivors {
            handle
                .await
                .expect("surviving waiter task")
                .expect("surviving waiter must check out after cancellations");
        }
    })
    .await
    .expect("cancelled waiters must not strand surviving checkouts");

    assert_eq!(
        spawned.load(Ordering::SeqCst),
        0,
        "the idle client is always returned before the slot frees, so no waiter may spawn"
    );
    assert_eq!(pool.slots.available_permits(), 1);
}

#[tokio::test]
async fn nested_pool_drain_during_client_start_rejects_the_stale_client() {
    let pool = leaked_test_pool(1);
    let result =
        checkout_nested_copilot_client_from(pool, CancellationToken::new(), move || async move {
            // A backend stop lands while the replacement CLI process is still starting.
            assert!(pool.drain().is_empty());
            Ok(unstarted_pool_client())
        })
        .await;
    let error = match result {
        Ok(_) => panic!("a client started across a drain must not join the pool"),
        Err(error) => error,
    };
    assert!(error.contains("drained"), "unexpected error: {error}");
    assert!(
        pool.drain().is_empty(),
        "the stale client must not be registered into the drained pool"
    );
    assert_eq!(
        pool.slots.available_permits(),
        1,
        "the rejected checkout must release its slot"
    );
}

#[tokio::test]
async fn nested_checkout_fails_fast_after_backend_stop_clears_start_options() {
    {
        let mut opts = COPILOT_START_OPTIONS.lock().await;
        *opts = Some(FlowPilotBackendStartOptions {
            use_stdio: true,
            cli_url: None,
            app_handle: None,
        });
    }
    assert!(nested_copilot_start_options().await.is_ok());

    // Backend stop clears the stored options alongside draining the nested pool.
    COPILOT_START_OPTIONS.lock().await.take();

    let pool = leaked_test_pool(1);
    let result = checkout_nested_copilot_client_from(pool, CancellationToken::new(), || async {
        nested_copilot_start_options().await?;
        Err::<Arc<Client>, String>("a post-stop checkout must not reach client startup".to_string())
    })
    .await;
    let error = match result {
        Ok(_) => panic!("checkout after backend stop must fail fast"),
        Err(error) => error,
    };
    assert!(error.contains("not started"), "unexpected error: {error}");
    assert_eq!(
        pool.slots.available_permits(),
        1,
        "the failed checkout must release its slot"
    );
}

#[tokio::test]
async fn nested_pool_drained_lease_is_not_returned_to_idle() {
    let pool = leaked_test_pool(1);
    let seeded = unstarted_pool_client();
    assert!(pool.register_started(seeded.clone(), pool.epoch()));
    pool.return_to_idle(seeded.clone());

    let lease = checkout_nested_copilot_client_from(pool, CancellationToken::new(), || async {
        Err::<Arc<Client>, String>("factory must not run for an idle checkout".to_string())
    })
    .await
    .expect("seeded checkout");

    let stopped = pool.drain();
    assert_eq!(stopped.len(), 1);
    assert!(Arc::ptr_eq(&stopped[0], &seeded));

    drop(lease);
    assert!(
        pool.take_idle().is_none(),
        "a lease drained mid-run was force-stopped by the backend stop path and must not rejoin idle"
    );
    assert_eq!(pool.slots.available_permits(), 1);
}

#[test]
fn direct_sdk_handlers_inherit_cancellation_without_an_overall_deadline() {
    let observed_no_deadline = Arc::new(StdMutex::new(false));
    let observed = observed_no_deadline.clone();
    let handler: copilot_sdk::ToolHandler = Arc::new(move |_name, _args| {
        let (cancellation, deadline) =
            crate::functions::ai::frontend_tool_bridge::current_tool_execution_for_test()
                .expect("SDK handler must inherit the frontend execution scope");
        *observed.lock().expect("observation lock") = deadline.is_none();
        cancellation.cancel();
        copilot_sdk::ToolResultObject::text("ok")
    });
    let cancellation = CancellationToken::new();
    let mut tools = scope_sdk_tool_handlers(
        vec![(copilot_sdk::Tool::new("runtime_test"), handler)],
        cancellation.clone(),
        None,
    );
    let (_, handler) = tools.pop().expect("scoped SDK handler");

    let result = handler("runtime_test", &serde_json::json!({}));

    assert_eq!(result.text_result_for_llm, "ok");
    assert!(
        *observed_no_deadline.lock().expect("observation lock"),
        "an active SDK provider run must not inherit an arbitrary wall-clock deadline"
    );
    assert!(
        cancellation.is_cancelled(),
        "the scoped handler must receive the owning run token, not a detached token"
    );
    assert!(
        crate::functions::ai::frontend_tool_bridge::current_tool_execution_for_test().is_none(),
        "the synchronous SDK scope must be restored after the handler returns"
    );
}

#[test]
fn cancelled_direct_sdk_handler_never_starts() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = calls.clone();
    let handler: copilot_sdk::ToolHandler = Arc::new(move |_name, _args| {
        handler_calls.fetch_add(1, Ordering::SeqCst);
        copilot_sdk::ToolResultObject::text("unexpected")
    });
    let cancellation = CancellationToken::new();
    let mut tools = scope_sdk_tool_handlers(
        vec![(copilot_sdk::Tool::new("runtime_test"), handler)],
        cancellation.clone(),
        None,
    );
    cancellation.cancel();
    let (_, handler) = tools.pop().expect("scoped SDK handler");

    let result = handler("runtime_test", &serde_json::json!({}));

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or(&result.text_result_for_llm)
            .contains("cancelled")
    );
}

#[test]
fn panicking_direct_sdk_handler_returns_an_error_and_releases_its_activity_lease() {
    let activity = Arc::new(SdkToolActivityRegistry::default());
    let observed_activity = activity.clone();
    let handler: copilot_sdk::ToolHandler = Arc::new(move |_name, _args| {
        assert_eq!(
            observed_activity
                .state
                .lock()
                .expect("activity state")
                .active_deadlines
                .len(),
            1,
            "the host-side lease must exist before the SDK publishes its tool event"
        );
        panic!("simulated handler panic");
    });
    let mut tools = scope_sdk_tool_handlers(
        vec![(copilot_sdk::Tool::new("ask_user"), handler)],
        CancellationToken::new(),
        Some(activity.clone()),
    );
    let (_, handler) = tools.pop().expect("scoped SDK handler");

    let result = handler("ask_user", &serde_json::json!({}));

    assert!(
        result
            .error
            .as_deref()
            .unwrap_or(&result.text_result_for_llm)
            .contains("simulated handler panic")
    );
    assert!(
        activity
            .state
            .lock()
            .expect("activity state")
            .active_deadlines
            .is_empty(),
        "panic unwinding must not leave the SDK watchdog extended"
    );
}
