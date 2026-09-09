use chrono::DateTime;
use kotoba_core::*;

fn at(s: &str) -> i64 {
    DateTime::parse_from_rfc3339(s).unwrap().timestamp()
}
fn setup() -> (Library, String, i64) {
    let mut lib = Library::new("Asia/Tokyo").unwrap();
    let now = at("2026-09-09T23:00:00+09:00");
    let deck = lib.create_deck("Study", now).unwrap();
    let csv = parse_csv(b"q,a\nQ1,A1\nQ2,A2\nQ3,A3\n", "utf-8").unwrap();
    lib.apply_import(
        lib.preview_import(
            &deck,
            &csv,
            Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
            },
            now,
        )
        .unwrap(),
    )
    .unwrap();
    (lib, deck, now)
}

#[test]
fn fsrs_grading_and_undo_restore_memory_and_daily_allowance() {
    let (mut lib, deck, now) = setup();
    lib.configure_deck(&deck, 0.9, 1, None).unwrap();
    let id = lib.queue(&deck, now).unwrap().card_ids[0].clone();
    lib.grade(&id, 3, now).unwrap();
    let card = lib.state.cards.iter().find(|c| c.id == id).unwrap();
    assert!((card.schedule.stability.unwrap() - 2.3065).abs() < 0.0001);
    assert_eq!(card.schedule.due, Some(now + 2 * 86400));
    assert_eq!(lib.queue(&deck, now).unwrap().new_used, 1);
    assert!(lib.queue(&deck, now).unwrap().card_ids.is_empty());
    assert_eq!(lib.state.reviews[0].algorithm, ALGORITHM);
    lib.undo().unwrap();
    assert_eq!(lib.queue(&deck, now).unwrap().new_used, 0);
    assert_eq!(lib.state.reviews.len(), 0);
    assert_eq!(
        lib.state
            .cards
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .schedule,
        Schedule::default()
    );
}

#[test]
fn bonus_survives_serialization_and_expires_at_four_without_resetting_normal_limit() {
    let (mut lib, deck, now) = setup();
    lib.configure_deck(&deck, 0.9, 1, None).unwrap();
    lib.add_bonus(&deck, 1, 0, now).unwrap();
    let q = lib.queue(&deck, now).unwrap();
    assert_eq!(q.card_ids.len(), 2);
    for id in q.card_ids {
        lib.grade(&id, 3, now).unwrap();
    }
    let lib = Library {
        state: serde_json::from_str(&serde_json::to_string(&lib.state).unwrap()).unwrap(),
    };
    let q = lib.queue(&deck, at("2026-09-10T03:59:59+09:00")).unwrap();
    assert_eq!(q.new_limit, 2);
    assert!(q.card_ids.is_empty());
    let q = lib.queue(&deck, at("2026-09-10T04:00:00+09:00")).unwrap();
    assert_eq!(q.new_limit, 1);
    assert_eq!(q.card_ids.len(), 1);
    assert_eq!(q.new_used, 0);
}

#[test]
fn relearning_does_not_consume_a_second_slot_and_backlog_stays_visible() {
    let (mut lib, deck, now) = setup();
    lib.configure_deck(&deck, 0.9, 1, Some(0)).unwrap();
    let id = lib.queue(&deck, now).unwrap().card_ids[0].clone();
    lib.grade(&id, 1, now).unwrap();
    assert!(lib.queue(&deck, now).unwrap().card_ids.is_empty());
    assert_eq!(
        lib.queue(&deck, now + 61).unwrap().card_ids,
        vec![id.clone()]
    );
    lib.grade(&id, 3, now + 61).unwrap();
    let q = lib.queue(&deck, now + 61).unwrap();
    assert_eq!((q.new_used, q.review_used), (1, 0));
    let later = now + 30 * 86400;
    let q = lib.queue(&deck, later).unwrap();
    assert_eq!(q.due_reviews, 1);
    assert!(!q.card_ids.contains(&id));
    lib.add_bonus(&deck, 0, 1, later).unwrap();
    assert_eq!(lib.queue(&deck, later).unwrap().card_ids[0], id);
}

#[test]
fn invalid_rating_or_future_review_never_changes_state_and_edit_retains_history() {
    let (mut lib, deck, now) = setup();
    let id = lib.queue(&deck, now).unwrap().card_ids[0].clone();
    let before = lib.state.clone();
    assert!(lib.grade(&id, 5, now).is_err());
    assert_eq!(before, lib.state);
    lib.grade(&id, 4, now).unwrap();
    let before = lib.state.clone();
    assert!(lib.grade(&id, 3, now + 1).is_err());
    assert_eq!(before, lib.state);
    lib.edit_card(&id, "Edited", "answer", "explanation")
        .unwrap();
    assert_eq!(lib.state.reviews, before.reviews);
    assert_eq!(lib.state.cards[0].schedule, before.cards[0].schedule);
    lib.set_settings(Settings {
        timezone: "Asia/Tokyo".into(),
        day_start_hour: 6,
    })
    .unwrap();
    assert_eq!(
        lib.queue(&deck, at("2026-09-10T05:59:00+09:00"))
            .unwrap()
            .new_used,
        1
    );
    assert!(lib.configure_deck(&deck, f32::NAN, 20, None).is_err());
}
