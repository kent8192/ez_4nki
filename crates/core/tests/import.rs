use kotoba_core::*;

fn mapping() -> Mapping {
    Mapping {
        question: 1,
        answer: 2,
        explanation: Some(3),
        id: Some(0),
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
                id: None
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
            },
            100,
        )
        .unwrap();
    assert_eq!(preview.errors[0].line, 4);
}
