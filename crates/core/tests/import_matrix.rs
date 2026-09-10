use kotoba_core::*;
use std::collections::{HashMap, HashSet};

const NOW: i64 = 1_789_000_000;
const HEADERS: [&str; 6] = ["ID", "問題", "答え", "解説", "選択肢", "未使用"];

fn shuffle<T>(items: &mut [T], seed: &mut u64) {
    for index in (1..items.len()).rev() {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        items.swap(index, (*seed as usize) % (index + 1));
    }
}

fn row(index: usize) -> Vec<String> {
    vec![
        format!("id,{index:04}"),
        format!("Q{index}\n\"架空の問題\""),
        format!("A{index}"),
        "架空の解説\n次の行".into(),
        if index.is_multiple_of(3) {
            String::new()
        } else {
            "one|two".into()
        },
        "NEVER_STORE_UNUSED_COLUMN".into(),
    ]
}

fn csv(rows: &[Vec<String>], order: &[usize]) -> ParsedCsv {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer
        .write_record(order.iter().map(|&i| HEADERS[i]))
        .unwrap();
    for row in rows {
        writer.write_record(order.iter().map(|&i| &row[i])).unwrap();
    }
    parse_csv(&writer.into_inner().unwrap(), "utf-8").unwrap()
}

fn mapping(order: &[usize], ids: bool) -> Mapping {
    let column = |original| order.iter().position(|&i| i == original).unwrap();
    Mapping {
        question: column(1),
        answer: column(2),
        explanation: Some(column(3)),
        id: ids.then(|| column(0)),
        choices: vec![column(4)],
        choice_separator: Some("|".into()),
    }
}

fn contents(state: &Snapshot, deck: &str) -> (Vec<Deck>, Vec<Card>, Vec<Review>, Vec<Bonus>) {
    (
        state
            .decks
            .iter()
            .filter(|d| d.id == deck)
            .cloned()
            .collect(),
        state
            .cards
            .iter()
            .filter(|c| c.deck_id == deck)
            .cloned()
            .collect(),
        state
            .reviews
            .iter()
            .filter(|r| r.deck_id == deck)
            .cloned()
            .collect(),
        state
            .bonuses
            .iter()
            .filter(|b| b.deck_id == deck)
            .cloned()
            .collect(),
    )
}

fn exercise_sequence(size: usize, mut seed: u64, ids: bool) -> Snapshot {
    let mut lib = Library::new("Asia/Tokyo").unwrap();
    let mut order: Vec<_> = (0..6).collect();
    shuffle(&mut order, &mut seed);
    let initial: Vec<_> = (0..size).map(row).collect();
    // Names, source IDs and questions all overlap. Only the internal deck ID selects a deck.
    let decks: Vec<_> = (0..3)
        .map(|_| lib.create_deck("同じ名前の単語帳", NOW).unwrap())
        .collect();
    for deck in &decks {
        if !initial.is_empty() {
            lib.apply_import(
                lib.preview_import(deck, &csv(&initial, &order), mapping(&order, ids), NOW)
                    .unwrap(),
            )
            .unwrap();
        }
        lib.configure_deck(deck, 0.85, 1000, Some(80)).unwrap();
        lib.add_bonus(deck, 5, 10, NOW).unwrap();
        for card in lib.queue(deck, NOW).unwrap().card_ids.into_iter().take(2) {
            lib.grade(&card, 3, NOW).unwrap();
        }
    }
    let before = lib.state.clone();
    let target = &decks[1];
    let original: HashMap<_, _> = lib
        .state
        .cards
        .iter()
        .filter(|c| &c.deck_id == target)
        .map(|c| (c.id.clone(), c.clone()))
        .collect();
    let mut updated: Vec<_> = initial
        .into_iter()
        .enumerate()
        .filter_map(|(i, mut row)| {
            if i.is_multiple_of(5) {
                return None;
            }
            row[2] = format!("Updated answer {i}");
            if ids && i.is_multiple_of(3) {
                row[1] = format!("Edited question {i}");
            }
            if i.is_multiple_of(2) {
                row[4] = "one|three".into();
            }
            Some(row)
        })
        .collect();
    updated.push(row(size + 1));
    updated.extend(updated.clone());
    shuffle(&mut updated, &mut seed);
    shuffle(&mut order, &mut seed);
    let source = csv(&updated, &order);
    let m = mapping(&order, ids);
    let preview = lib
        .preview_import(target, &source, m.clone(), NOW + 1)
        .unwrap();
    assert!(preview.errors.is_empty(), "{:?}", preview.errors);
    assert_eq!(
        preview.changes.iter().filter(|c| c.kind == "new").count(),
        1
    );
    assert_eq!(preview.missing, size.div_ceil(5));
    assert_eq!(preview.merged_rows, updated.len() / 2);
    lib.apply_import(preview).unwrap();
    assert_eq!(lib.state.reviews, before.reviews);
    assert_eq!(lib.state.settings, before.settings);
    assert_eq!(
        contents(&lib.state, &decks[0]),
        contents(&before, &decks[0])
    );
    assert_eq!(
        contents(&lib.state, &decks[2]),
        contents(&before, &decks[2])
    );
    let actual: Vec<_> = lib
        .state
        .cards
        .iter()
        .filter(|c| &c.deck_id == target)
        .collect();
    assert_eq!(actual.len(), size + 1);
    for (id, old) in original {
        let current = actual
            .iter()
            .find(|c| c.id == id)
            .expect("existing card must remain");
        assert_eq!(current.schedule, old.schedule);
        assert_eq!(current.source_id, old.source_id);
        let expected = updated.iter().find(|r| {
            if ids {
                Some(&r[0]) == old.source_id.as_ref()
            } else {
                r[1] == old.question
            }
        });
        if let Some(expected) = expected {
            assert_eq!(current.question, expected[1]);
            assert_eq!(current.answer, expected[2]);
        } else {
            assert_eq!(
                **current, old,
                "a card missing from the CSV must be retained intact"
            );
        }
    }
    assert!(
        !serde_json::to_string(&lib.state)
            .unwrap()
            .contains("NEVER_STORE_UNUSED_COLUMN")
    );
    validate_snapshot(&lib.state).unwrap();
    let before_repeat = lib.state.clone();
    let repeat = lib
        .preview_import(target, &source, m.clone(), NOW + 2)
        .unwrap();
    assert!(repeat.changes.iter().all(|c| c.kind == "unchanged"));
    lib.apply_import(repeat).unwrap();
    assert_eq!(lib.state.cards, before_repeat.cards);
    assert_eq!(lib.state.reviews, before_repeat.reviews);
    let before_invalid = lib.state.clone();
    updated[0][if ids { 0 } else { 2 }].clear();
    let invalid = lib
        .preview_import(target, &csv(&updated, &order), m, NOW + 3)
        .unwrap();
    assert!(!invalid.errors.is_empty());
    assert!(lib.apply_import(invalid).is_err());
    assert_eq!(lib.state, before_invalid);
    lib.state
}

#[test]
fn seeded_reimport_sequences_preserve_identity_history_and_deck_isolation() {
    for ids in [false, true] {
        for seed in 1..=512 {
            let result =
                std::panic::catch_unwind(|| exercise_sequence((seed % 16 + 1) as usize, seed, ids));
            assert!(result.is_ok(), "replay with seed={seed}, ids={ids}");
        }
    }
}

#[test]
fn empty_small_and_near_limit_decks_survive_reimport_and_sqlite_restart() {
    for ids in [false, true] {
        for size in [0, 1, 2, 17, 120, 999] {
            let state = exercise_sequence(size, 73, ids);
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("library.sqlite3");
            let mut db = Database::open(&path, "UTC").unwrap();
            db.save(&state, 0).unwrap();
            drop(db);
            assert_eq!(
                Database::open(&path, "UTC").unwrap().load().unwrap(),
                state,
                "size={size}, ids={ids}"
            );
        }
    }
}

#[test]
fn ids_are_exact_strings_and_are_not_coerced_or_normalized() {
    let keys = [
        "1",
        "01",
        " 01",
        "01 ",
        "A",
        "a",
        "é",
        "e\u{301}",
        "１",
        "key,quoted",
        "key\nnext",
        "key\r\nnext",
        "\"key\"",
    ];
    let mut rows: Vec<_> = keys
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let mut row = row(i);
            row[0] = (*key).into();
            row[1] = "同じ問題文".into();
            row
        })
        .collect();
    let order: Vec<_> = (0..6).collect();
    let mut lib = Library::new("UTC").unwrap();
    let deck = lib.create_deck("Exact IDs", NOW).unwrap();
    lib.apply_import(
        lib.preview_import(&deck, &csv(&rows, &order), mapping(&order, true), NOW)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(lib.state.cards.len(), keys.len());
    let before: HashMap<_, _> = lib
        .state
        .cards
        .iter()
        .map(|c| (c.source_id.clone().unwrap(), c.id.clone()))
        .collect();
    assert_eq!(
        before.keys().cloned().collect::<HashSet<_>>(),
        keys.iter().map(|s| (*s).to_owned()).collect()
    );
    rows.reverse();
    for row in &mut rows {
        row[2] = "updated".into();
    }
    lib.apply_import(
        lib.preview_import(&deck, &csv(&rows, &order), mapping(&order, true), NOW + 1)
            .unwrap(),
    )
    .unwrap();
    for card in &lib.state.cards {
        assert_eq!(card.id, before[card.source_id.as_ref().unwrap()]);
        assert_eq!(card.answer, "updated");
    }
}

#[test]
fn ambiguous_existing_questions_or_legacy_ids_block_the_entire_import() {
    for duplicate_ids in [false, true] {
        let order: Vec<_> = (0..6).collect();
        let mut rows = vec![row(0), row(1), row(2)];
        rows[1][1] = rows[0][1].clone();
        let mut lib = Library::new("UTC").unwrap();
        let deck = lib.create_deck("Ambiguous", NOW).unwrap();
        lib.apply_import(
            lib.preview_import(&deck, &csv(&rows, &order), mapping(&order, true), NOW)
                .unwrap(),
        )
        .unwrap();
        for id in lib.queue(&deck, NOW).unwrap().card_ids {
            lib.grade(&id, 3, NOW).unwrap();
        }
        if duplicate_ids {
            // A legacy snapshot may contain duplicate source IDs but distinct internal IDs and histories.
            lib.state.cards[1].source_id = lib.state.cards[0].source_id.clone();
        }
        validate_snapshot(&lib.state).unwrap();
        let before = lib.state.clone();
        rows[0][2] = "must not apply".into();
        rows[2][2] = "nor this otherwise valid change".into();
        rows.remove(1);
        let preview = lib
            .preview_import(
                &deck,
                &csv(&rows, &order),
                mapping(&order, duplicate_ids),
                NOW + 1,
            )
            .unwrap();
        assert!(
            preview
                .errors
                .iter()
                .any(|e| e.message.contains("既存カードの候補が複数"))
        );
        assert_eq!(preview.changes.len(), 1);
        assert!(lib.apply_import(preview).is_err());
        assert_eq!(lib.state, before);
    }
}

#[test]
fn deleting_and_recreating_a_named_deck_does_not_reuse_old_card_history() {
    let mut lib = Library::new("UTC").unwrap();
    let first = lib.create_deck("Same name", NOW).unwrap();
    let order: Vec<_> = (0..6).collect();
    let source = csv(&[row(0)], &order);
    lib.apply_import(
        lib.preview_import(&first, &source, mapping(&order, true), NOW)
            .unwrap(),
    )
    .unwrap();
    let card = lib.state.cards[0].id.clone();
    lib.grade(&card, 3, NOW).unwrap();
    lib.add_bonus(&first, 5, 10, NOW).unwrap();
    let stale = lib
        .preview_import(&first, &source, mapping(&order, true), NOW + 1)
        .unwrap();
    lib.delete_deck(&first).unwrap();
    let second = lib.create_deck("Same name", NOW + 1).unwrap();
    let before = lib.state.clone();
    assert!(lib.apply_import(stale).is_err());
    assert!(
        lib.preview_import(&first, &source, mapping(&order, true), NOW + 1)
            .is_err()
    );
    assert!(lib.delete_deck(&first).is_err());
    assert_eq!(lib.state, before);
    lib.apply_import(
        lib.preview_import(&second, &source, mapping(&order, true), NOW + 1)
            .unwrap(),
    )
    .unwrap();
    assert_ne!(lib.state.cards[0].id, card);
    assert_eq!(lib.state.cards[0].schedule, Schedule::default());
    assert!(lib.state.reviews.is_empty());
    assert!(lib.state.bonuses.is_empty());
    validate_snapshot(&lib.state).unwrap();
}

#[test]
fn invalid_column_assignments_and_row_widths_never_modify_a_deck() {
    let mut lib = Library::new("UTC").unwrap();
    let deck = lib.create_deck("Columns", NOW).unwrap();
    let order: Vec<_> = (0..6).collect();
    let mut source = csv(&[row(0)], &order);
    let before = lib.state.clone();
    for broken in [
        Mapping {
            id: Some(usize::MAX),
            ..mapping(&order, true)
        },
        Mapping {
            answer: 1,
            ..mapping(&order, true)
        },
        Mapping {
            choices: vec![4, 4],
            ..mapping(&order, true)
        },
    ] {
        assert!(lib.preview_import(&deck, &source, broken, NOW).is_err());
    }
    source.rows[0].pop();
    let preview = lib
        .preview_import(&deck, &source, mapping(&order, true), NOW)
        .unwrap();
    assert_eq!(preview.errors[0].line, 2);
    assert!(lib.apply_import(preview).is_err());
    assert_eq!(lib.state, before);
}
