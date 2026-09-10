use kotoba_core::*;

fn setup() -> (Library, String) {
    let mut lib = Library::new("UTC").unwrap();
    let deck = lib.create_deck("Test", 1000).unwrap();
    let csv = parse_csv(b"q,a\nQ1,A1\nQ2,A2\n", "utf-8").unwrap();
    lib.apply_import(
        lib.preview_import(
            &deck,
            &csv,
            Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
                choices: vec![],
                choice_separator: None,
            },
            1000,
        )
        .unwrap(),
    )
    .unwrap();
    (lib, deck)
}

#[test]
fn unlearned_cards_have_no_curve_and_limited_data_does_not_claim_personal_fit() {
    let (lib, deck) = setup();
    assert!(lib.curve(&lib.state.cards[0].id, 1000).unwrap().is_empty());
    let analytics = lib.analytics(&deck, 1000).unwrap();
    assert!(!analytics.can_optimize);
    assert_eq!(analytics.forecast.len(), 30);
    assert_eq!(analytics.success_rate, None);
    assert!(
        lib.optimize(&deck, fsrs::CombinedProgressState::new_shared())
            .is_err()
    );
}

#[test]
fn forgetting_curve_declines_and_parameter_changes_keep_existing_due_dates() {
    let (mut lib, deck) = setup();
    let id = lib.state.cards[0].id.clone();
    lib.grade(&id, 3, 1000).unwrap();
    let curve = lib.curve(&id, 1000).unwrap();
    assert_eq!(curve[0].retention, 1.0);
    assert!(curve.windows(2).all(|p| p[0].retention >= p[1].retention));
    let due = lib.state.cards[0].schedule.due;
    let mut parameters = fsrs::DEFAULT_PARAMETERS.to_vec();
    parameters[0] += 0.01;
    lib.apply_parameters(&deck, parameters, lib.state.revision)
        .unwrap();
    assert_eq!(lib.state.cards[0].schedule.due, due);
    assert_eq!(lib.analytics(&deck, 1000).unwrap().success_rate, Some(1.0));
}

#[test]
fn training_uses_each_history_prefix_and_card_groups_without_exporting_content() {
    let (mut lib, deck) = setup();
    let id = lib.state.cards[0].id.clone();
    lib.grade(&id, 1, 1000).unwrap();
    lib.grade(&id, 3, 1061).unwrap();
    let due = lib.state.cards[0].schedule.due.unwrap();
    lib.grade(&id, 3, due).unwrap();
    let (items, ids) = lib.training_items(&deck).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].reviews.len(), 3);
    assert_eq!(items[0].reviews[2].delta_t, 1);
    assert_eq!(items[0].reviews[1].delta_t, 0);
    assert_eq!(ids, vec![0]);
}

#[test]
fn forecast_respects_card_limits_without_an_unrequested_time_budget() {
    let mut lib = Library::new("UTC").unwrap();
    let deck = lib.create_deck("Large synthetic deck", 1000).unwrap();
    let mut text = String::from("q,a\n");
    for i in 0..999 {
        text.push_str(&format!("Question {i},Answer {i}\n"));
    }
    let csv = parse_csv(text.as_bytes(), "utf-8").unwrap();
    lib.apply_import(
        lib.preview_import(
            &deck,
            &csv,
            Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
                choices: vec![],
                choice_separator: None,
            },
            1000,
        )
        .unwrap(),
    )
    .unwrap();
    lib.configure_deck(&deck, 0.9, 999, None).unwrap();
    assert_eq!(
        lib.analytics(&deck, 1000).unwrap().forecast[0].new_cards,
        999
    );
}

#[test]
fn real_local_optimizer_completes_and_cancelled_or_stale_work_cannot_change_schedules() {
    let mut lib = Library::new("UTC").unwrap();
    let deck = lib
        .create_deck("Synthetic optimization", 1_700_000_000)
        .unwrap();
    let mut text = String::from("q,a\n");
    for i in 0..80 {
        text.push_str(&format!("Q{i},A{i}\n"));
    }
    let csv = parse_csv(text.as_bytes(), "utf-8").unwrap();
    lib.apply_import(
        lib.preview_import(
            &deck,
            &csv,
            Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
                choices: vec![],
                choice_separator: None,
            },
            1_700_000_000,
        )
        .unwrap(),
    )
    .unwrap();
    lib.configure_deck(&deck, 0.9, 80, None).unwrap();
    for i in 0..80 {
        lib.grade(
            &lib.state.cards[i].id.clone(),
            (i % 4 + 1) as u32,
            1_700_000_000,
        )
        .unwrap();
    }
    let mut samples = 0;
    for i in 0..2000 {
        let card = lib
            .state
            .cards
            .iter()
            .min_by_key(|c| c.schedule.due)
            .unwrap();
        let (id, due) = (card.id.clone(), card.schedule.due.unwrap());
        lib.grade(&id, if i % 7 == 0 { 1 } else { (i % 3 + 2) as u32 }, due)
            .unwrap();
        if lib.state.reviews.last().unwrap().elapsed_days > 0 {
            samples += 1;
        }
        if samples == 600 {
            break;
        }
    }
    assert_eq!(samples, 600);
    validate_snapshot(&lib.state).unwrap();
    let before = lib.state.clone();
    let cancelled = fsrs::CombinedProgressState::new_shared();
    cancelled.lock().unwrap().want_abort = true;
    assert!(lib.optimize(&deck, cancelled).is_err());
    assert_eq!(lib.state, before);
    let progress = fsrs::CombinedProgressState::new_shared();
    let parameters = lib.optimize(&deck, progress.clone()).unwrap();
    assert!(
        progress.lock().unwrap().total() > 0,
        "the fixture must exercise training, not just initialization"
    );
    assert_ne!(parameters[4..], fsrs::DEFAULT_PARAMETERS[4..]);
    assert_eq!(parameters.len(), 21);
    assert!(parameters.iter().all(|v| v.is_finite()));
    assert!(
        lib.apply_parameters(&deck, parameters.clone(), before.revision - 1)
            .is_err()
    );
    assert_eq!(lib.state, before);
    lib.apply_parameters(&deck, parameters, before.revision)
        .unwrap();
    assert_eq!(lib.state.cards, before.cards);
    assert_eq!(lib.state.reviews, before.reviews);
}
