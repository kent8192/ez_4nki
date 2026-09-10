use kotoba_core::*;

fn mapping() -> Mapping {
    Mapping {
        question: 1,
        answer: 2,
        explanation: Some(3),
        id: Some(0),
        choices: vec![],
        choice_separator: None,
    }
}
fn library() -> (Library, String) {
    let mut lib = Library::new("Asia/Tokyo").unwrap();
    let id = lib.create_deck("算数", 1_789_000_000).unwrap();
    (lib, id)
}

#[test]
fn arbitrary_columns_and_multiline_text_create_cards_without_unused_data() {
    let (mut lib, id) = library();
    let csv = parse_csv(
        include_bytes!("../../../docs/examples/initial.csv"),
        "utf-8",
    )
    .unwrap();
    let preview = lib
        .preview_import(&id, &csv, mapping(), 1_789_000_000)
        .unwrap();
    assert_eq!(preview.changes.len(), 3);
    assert!(preview.errors.is_empty());
    lib.apply_import(preview).unwrap();
    assert_eq!(lib.state.cards.len(), 3);
    assert!(
        lib.state
            .cards
            .iter()
            .find(|c| c.source_id.as_deref() == Some("002"))
            .unwrap()
            .explanation
            .contains('\n')
    );
    assert!(
        !serde_json::to_string(&lib.state)
            .unwrap()
            .contains("架空の補助情報")
    );
}

#[test]
fn reimport_preserves_card_identity_schedule_and_missing_cards() {
    let (mut lib, id) = library();
    let csv = parse_csv(
        include_bytes!("../../../docs/examples/initial.csv"),
        "utf-8",
    )
    .unwrap();
    lib.apply_import(lib.preview_import(&id, &csv, mapping(), 100).unwrap())
        .unwrap();
    let original = lib.state.cards[0].id.clone();
    lib.state.cards[0].schedule.reps = 5;
    let csv = parse_csv(
        include_bytes!("../../../docs/examples/updated.csv"),
        "utf-8",
    )
    .unwrap();
    let p = lib
        .preview_import(
            &id,
            &csv,
            Mapping {
                question: 2,
                answer: 1,
                explanation: Some(0),
                id: Some(3),
                choices: vec![],
                choice_separator: None,
            },
            200,
        )
        .unwrap();
    assert_eq!(p.missing, 1);
    assert_eq!(p.changes.iter().filter(|c| c.kind == "update").count(), 1);
    assert_eq!(p.changes.iter().filter(|c| c.kind == "new").count(), 1);
    assert_eq!(
        p.changes.iter().filter(|c| c.kind == "unchanged").count(),
        1
    );
    lib.apply_import(p).unwrap();
    let card = lib.state.cards.iter().find(|c| c.id == original).unwrap();
    assert_eq!(card.schedule.reps, 5);
    assert_eq!(card.question, "2と3の合計は？");
    assert_eq!(lib.state.cards.len(), 4);
}

#[test]
fn ambiguous_and_stale_imports_leave_existing_cards_untouched() {
    let (mut lib, id) = library();
    let csv = parse_csv(
        include_bytes!("../../../docs/examples/ambiguous.csv"),
        "utf-8",
    )
    .unwrap();
    let p = lib.preview_import(&id, &csv, mapping(), 100).unwrap();
    assert!(!p.errors.is_empty());
    let before = lib.state.clone();
    assert!(lib.apply_import(p).is_err());
    assert_eq!(lib.state, before);
    let csv = parse_csv(
        include_bytes!("../../../docs/examples/initial.csv"),
        "utf-8",
    )
    .unwrap();
    let p = lib.preview_import(&id, &csv, mapping(), 100).unwrap();
    lib.create_deck("another", 200).unwrap();
    let before = lib.state.clone();
    assert!(lib.apply_import(p).is_err());
    assert_eq!(lib.state, before);
}

#[test]
fn question_matching_is_exact_and_more_than_999_rows_are_not_truncated() {
    let (mut lib, id) = library();
    let source = format!(
        "q,a\n{}",
        (0..1001)
            .map(|n| format!("Q{n},A{n}\n"))
            .collect::<String>()
    );
    let csv = parse_csv(source.as_bytes(), "utf-8").unwrap();
    let m = Mapping {
        question: 0,
        answer: 1,
        explanation: None,
        id: None,
        choices: vec![],
        choice_separator: None,
    };
    lib.apply_import(lib.preview_import(&id, &csv, m.clone(), 100).unwrap())
        .unwrap();
    assert_eq!(lib.state.cards.len(), 1001);
    let csv = parse_csv(b"q,a\nQ0,changed\n Q1,spaced\n", "utf-8").unwrap();
    let p = lib.preview_import(&id, &csv, m, 200).unwrap();
    assert_eq!(p.changes[0].kind, "update");
    assert_eq!(p.changes[1].kind, "new");
}

#[test]
fn duplicate_questions_merge_distinct_content_and_preserve_review_history() {
    let (mut lib, id) = library();
    let m = Mapping {
        question: 0,
        answer: 1,
        explanation: Some(2),
        id: None,
        choices: vec![],
        choice_separator: None,
    };
    let csv = parse_csv(
        "問題,答え,解説\nbank,銀行,金融機関\nbank,銀行,金融機関\nbank,土手,川沿いの土地\n"
            .as_bytes(),
        "utf-8",
    )
    .unwrap();
    let preview = lib
        .preview_import(&id, &csv, m.clone(), 1_789_000_000)
        .unwrap();
    assert!(preview.errors.is_empty(), "{:?}", preview.errors);
    assert_eq!(preview.changes.len(), 1);
    assert_eq!(preview.merged_rows, 2);
    assert_eq!(preview.changes[0].source_lines, vec![2, 3, 4]);
    assert_eq!(preview.changes[0].notes.len(), 2);
    assert_eq!(preview.changes[0].after.answer, "銀行\n\n土手");
    assert_eq!(
        preview.changes[0].after.explanation,
        "金融機関\n\n川沿いの土地"
    );
    lib.apply_import(preview).unwrap();
    let card_id = lib.state.cards[0].id.clone();
    lib.grade(&card_id, 3, 1_789_000_000).unwrap();
    let before = lib.state.clone();
    let preview = lib.preview_import(&id, &csv, m, 1_789_000_100).unwrap();
    assert_eq!(preview.changes[0].kind, "unchanged");
    lib.apply_import(preview).unwrap();
    assert_eq!(lib.state.cards, before.cards);
    assert_eq!(lib.state.reviews, before.reviews);
}

#[test]
fn a_120_row_csv_with_repeated_questions_becomes_100_cards() {
    let (mut lib, id) = library();
    let source = format!(
        "q,a\n{}",
        (0..120)
            .map(|n| format!("Q{},A{}\n", n % 100, n % 100))
            .collect::<String>()
    );
    let csv = parse_csv(source.as_bytes(), "utf-8").unwrap();
    let preview = lib
        .preview_import(
            &id,
            &csv,
            Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
                choices: vec![],
                choice_separator: None,
            },
            100,
        )
        .unwrap();
    assert!(preview.errors.is_empty());
    assert_eq!(preview.merged_rows, 20);
    assert_eq!(preview.changes.len(), 100);
    lib.apply_import(preview).unwrap();
    assert_eq!(lib.state.cards.len(), 100);
}

#[test]
fn single_column_choices_preserve_blocks_or_split_explicitly_and_merge_in_order() {
    let (mut lib, id) = library();
    let csv = parse_csv(b"q,a,options,unused\nQ,A,\"A. one\r\nB. two\n\",SECRET_UNUSED\nQ,A,\"B. two\nC. three\",SECRET_UNUSED\n", "utf-8").unwrap();
    let mut m = Mapping {
        question: 0,
        answer: 1,
        explanation: None,
        id: None,
        choices: vec![2],
        choice_separator: None,
    };
    let preview = lib.preview_import(&id, &csv, m.clone(), 100).unwrap();
    assert!(preview.errors.is_empty());
    assert_eq!(preview.changes[0].source_lines, vec![2, 5]);
    assert_eq!(
        preview.changes[0].after.choices[0],
        Choice {
            label: "options".into(),
            text: "A. one\r\nB. two\n".into(),
        }
    );
    m.choice_separator = Some("\n".into());
    let preview = lib.preview_import(&id, &csv, m.clone(), 100).unwrap();
    assert_eq!(
        preview.changes[0]
            .after
            .choices
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>(),
        vec!["A. one", "B. two", "C. three"]
    );
    lib.apply_import(preview).unwrap();
    assert!(
        !serde_json::to_string(&lib.state)
            .unwrap()
            .contains("SECRET_UNUSED")
    );
    let card = lib.state.cards[0].id.clone();
    lib.grade(&card, 3, 100).unwrap();
    let before = lib.state.clone();
    let updated = parse_csv(b"q,a,options\nQ,A,\"A. one\nD. four\"\n", "utf-8").unwrap();
    let preview = lib.preview_import(&id, &updated, m.clone(), 101).unwrap();
    assert_eq!(preview.changes[0].kind, "update");
    lib.apply_import(preview).unwrap();
    assert_eq!(lib.state.cards[0].id, card);
    assert_eq!(lib.state.cards[0].schedule, before.cards[0].schedule);
    assert_eq!(lib.state.reviews, before.reviews);
    assert_eq!(lib.state.cards[0].choices[1].text, "D. four");
    assert_eq!(
        lib.preview_import(&id, &updated, m, 102).unwrap().changes[0].kind,
        "unchanged"
    );
}

#[test]
fn custom_choice_separators_are_literal_and_column_roles_cannot_overlap() {
    let (lib, id) = library();
    let csv = parse_csv(
        b"q,a,options,extra\nQ,A,A. one||B. two|||B. two|,Other\n",
        "utf-8",
    )
    .unwrap();
    let m = Mapping {
        question: 0,
        answer: 1,
        explanation: None,
        id: None,
        choices: vec![2],
        choice_separator: Some("|".into()),
    };
    let preview = lib.preview_import(&id, &csv, m.clone(), 100).unwrap();
    assert_eq!(preview.changes[0].after.choices.len(), 2);
    assert_eq!(preview.changes[0].after.choices[1].text, "B. two");
    for choices in [vec![0], vec![2, 2], vec![4]] {
        assert!(
            lib.preview_import(
                &id,
                &csv,
                Mapping {
                    choices,
                    ..m.clone()
                },
                100
            )
            .is_err()
        );
    }
    assert!(
        lib.preview_import(
            &id,
            &csv,
            Mapping {
                choice_separator: Some(String::new()),
                ..m.clone()
            },
            100
        )
        .is_err()
    );
    let preview = lib
        .preview_import(
            &id,
            &csv,
            Mapping {
                choices: vec![3, 2],
                choice_separator: None,
                ..m
            },
            100,
        )
        .unwrap();
    assert_eq!(preview.changes[0].after.choices[0].label, "options");
    assert_eq!(preview.changes[0].after.choices[1].label, "extra");
}

#[test]
fn repeated_ids_merge_the_same_question_but_conflicting_questions_still_block_apply() {
    let (mut lib, id) = library();
    let csv = parse_csv(b"id,q,a,e\n1,Q,A,E\n1,Q,A,E\n2,Q,A,E\n", "utf-8").unwrap();
    let preview = lib.preview_import(&id, &csv, mapping(), 100).unwrap();
    assert!(preview.errors.is_empty());
    assert_eq!(preview.changes.len(), 2);
    assert_eq!(preview.merged_rows, 1);
    lib.apply_import(preview).unwrap();
    let before = lib.state.clone();
    let csv = parse_csv(
        b"id,q,a,e\n1,Q,A,E\n1,Different,A,E\n1,Another,A,E\n",
        "utf-8",
    )
    .unwrap();
    let preview = lib.preview_import(&id, &csv, mapping(), 101).unwrap();
    assert_eq!(preview.errors.len(), 2);
    assert_eq!(preview.errors[0].line, 3);
    assert_eq!(preview.errors[1].line, 4);
    assert!(
        preview
            .errors
            .iter()
            .all(|e| e.message.starts_with("2行目と同じID"))
    );
    assert!(lib.apply_import(preview).is_err());
    assert_eq!(lib.state, before);
}

#[test]
fn cp932_and_bom_decode_but_invalid_required_fields_block_import() {
    let (lib, id) = library();
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("問題,答え\n挨拶,おはよう\n");
    assert_eq!(parse_csv(&bytes, "cp932").unwrap().rows[0][0], "挨拶");
    let csv = parse_csv("\u{feff}q,a\n,answer\n".as_bytes(), "utf-8").unwrap();
    assert_eq!(csv.headers[0], "q");
    assert!(
        !lib.preview_import(
            &id,
            &csv,
            Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
                choices: vec![],
                choice_separator: None,
            },
            100
        )
        .unwrap()
        .errors
        .is_empty()
    );
    assert!(parse_csv(&[0xff], "utf-8").is_err());
    assert!(parse_csv(b"q,a\nx,y,z\n", "utf-8").is_err());
}

#[test]
fn malformed_quotes_are_rejected_and_multiline_records_report_source_line_numbers() {
    assert!(parse_csv(b"q,a\n\"unfinished,answer\n", "utf-8").is_err());
    assert!(parse_csv(b"q,a\n\"closed\"junk,answer\n", "utf-8").is_err());
    let (lib, id) = library();
    let csv = parse_csv(b"q,a\n\"line1\nline2\",answer\n,last\n", "utf-8").unwrap();
    let preview = lib
        .preview_import(
            &id,
            &csv,
            Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
                choices: vec![],
                choice_separator: None,
            },
            100,
        )
        .unwrap();
    assert_eq!(preview.errors[0].line, 4);
}
