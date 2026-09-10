use super::*;

fn deck(app: &AppState, name: &str) -> String {
    app.handle(Command::CreateDeck { name: name.into() }, 100)
        .unwrap()
        .as_str()
        .unwrap()
        .into()
}

fn source(app: &AppState, text: &str) -> String {
    let token = uuid::Uuid::new_v4().to_string();
    *lock(&app.imports).unwrap() = ImportCache {
        source: Some((token.clone(), parse_csv(text.as_bytes(), "utf-8").unwrap())),
        preview: None,
    };
    token
}

fn mapping(ids: bool) -> Mapping {
    Mapping {
        question: 1,
        answer: 2,
        explanation: None,
        id: ids.then_some(0),
        choices: vec![],
        choice_separator: None,
    }
}

fn preview(app: &AppState, deck: &str, token: &str, mapping: Mapping) -> Result<Value> {
    app.handle(
        serde_json::from_value(json!({"type":"previewImport", "deckId":deck,
        "sourceToken":token, "mapping":mapping}))
        .unwrap(),
        101,
    )
}

fn apply(app: &AppState, candidate: &Value) -> Result<Value> {
    app.handle(
        serde_json::from_value(json!({"type":"applyImport", "token":candidate["token"]})).unwrap(),
        102,
    )
}

#[test]
fn failing_to_regenerate_a_preview_invalidates_the_previous_candidate() {
    let directory = tempfile::tempdir().unwrap();
    let app = AppState::new(directory.path().into()).unwrap();
    let deck = deck(&app, "Synthetic");
    let token = source(&app, "id,q,a\n1,Q,A\n");
    let valid = preview(&app, &deck, &token, mapping(true)).unwrap();
    let before = app.library().unwrap().state;
    assert!(
        preview(
            &app,
            &deck,
            &token,
            Mapping {
                id: Some(1),
                ..mapping(true)
            }
        )
        .is_err()
    );
    assert!(
        apply(&app, &valid).is_err(),
        "a failed new column assignment must invalidate the old preview"
    );
    assert_eq!(app.library().unwrap().state, before);
}

#[test]
fn shared_ids_or_questions_do_not_cross_decks_and_mapping_survives_restart() {
    for ids in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let app = AppState::new(directory.path().into()).unwrap();
        let left = deck(&app, "Same name");
        let right = deck(&app, "Same name");
        for deck in [&left, &right] {
            let token = source(&app, "id,q,a\n001,Same question,First answer\n");
            apply(&app, &preview(&app, deck, &token, mapping(ids)).unwrap()).unwrap();
            let id = app
                .library()
                .unwrap()
                .state
                .cards
                .iter()
                .find(|c| &c.deck_id == deck)
                .unwrap()
                .id
                .clone();
            app.handle(
                Command::Grade {
                    card_id: id,
                    rating: 3,
                },
                102,
            )
            .unwrap();
        }
        let before = app.library().unwrap().state;
        let token = source(&app, "id,q,a\n001,Same question,Updated answer\n");
        let candidate = preview(&app, &left, &token, mapping(ids)).unwrap();
        assert_eq!(candidate["changes"][0]["kind"], "update");
        apply(&app, &candidate).unwrap();
        assert!(apply(&app, &candidate).is_err());
        drop(app);
        let app = AppState::new(directory.path().into()).unwrap();
        let after = app.library().unwrap().state;
        assert_eq!(after.cards.len(), 2);
        assert_eq!(after.reviews, before.reviews);
        for (old, new) in before.cards.iter().zip(&after.cards) {
            assert_eq!(old.id, new.id);
            assert_eq!(old.schedule, new.schedule);
            if new.deck_id == left {
                assert_eq!(new.answer, "Updated answer");
            } else {
                assert_eq!(new, old);
            }
        }
        assert!(after.decks.iter().all(|d| d.mapping == Some(mapping(ids))));
    }
}

#[test]
fn only_the_latest_preview_of_the_selected_source_can_apply_to_its_own_deck() {
    let directory = tempfile::tempdir().unwrap();
    let app = AppState::new(directory.path().into()).unwrap();
    let left = deck(&app, "Same name");
    let right = deck(&app, "Same name");
    let token = source(&app, "id,q,a\n1,Q,A\n");
    let old = preview(&app, &left, &token, mapping(true)).unwrap();
    let current = preview(&app, &right, &token, mapping(true)).unwrap();
    assert!(apply(&app, &old).is_err());
    apply(&app, &current).unwrap();
    let state = app.library().unwrap().state;
    assert_eq!(state.cards.len(), 1);
    assert_eq!(state.cards[0].deck_id, right);
    let token = source(&app, "id,q,a\n1,Q,Second\n");
    let replaced = preview(&app, &right, &token, mapping(true)).unwrap();
    source(&app, "id,q,a\n2,Other,New source\n");
    assert!(apply(&app, &replaced).is_err());
    assert_eq!(app.library().unwrap().state, state);
}

#[test]
fn an_outdated_source_request_cannot_replace_a_valid_current_preview() {
    let directory = tempfile::tempdir().unwrap();
    let app = AppState::new(directory.path().into()).unwrap();
    let deck = deck(&app, "Synthetic");
    let old_source = source(&app, "id,q,a\n1,Old,A\n");
    let current_source = source(&app, "id,q,a\n2,Current,B\n");
    let current = preview(&app, &deck, &current_source, mapping(true)).unwrap();
    assert!(preview(&app, &deck, &old_source, mapping(true)).is_err());
    apply(&app, &current).unwrap();
    assert_eq!(app.library().unwrap().state.cards[0].question, "Current");
}

#[test]
fn content_errors_block_all_rows_but_an_explicit_question_preview_can_recover() {
    let directory = tempfile::tempdir().unwrap();
    let app = AppState::new(directory.path().into()).unwrap();
    let deck = deck(&app, "Synthetic");
    let token = source(&app, "id,q,a\n,Blank ID,A\n1,Q1,A\n1,Q2,B\n");
    let before = app.library().unwrap().state;
    let invalid = preview(&app, &deck, &token, mapping(true)).unwrap();
    assert_eq!(invalid["errors"].as_array().unwrap().len(), 2);
    assert!(apply(&app, &invalid).is_err());
    assert_eq!(app.library().unwrap().state, before);
    let valid = preview(&app, &deck, &token, mapping(false)).unwrap();
    assert!(valid["errors"].as_array().unwrap().is_empty());
    assert!(apply(&app, &invalid).is_err());
    apply(&app, &valid).unwrap();
    assert_eq!(app.library().unwrap().state.cards.len(), 3);
    assert_eq!(
        app.library().unwrap().state.decks[0].mapping,
        Some(mapping(false))
    );
}

#[test]
fn every_intervening_mutation_keeps_stale_imports_from_reverting_current_data() {
    for action in [
        "grade",
        "edit",
        "bonus",
        "configure",
        "settings",
        "create",
        "delete",
        "restore",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let app = AppState::new(directory.path().into()).unwrap();
        let deck = deck(&app, "Synthetic");
        let token = source(&app, "id,q,a\n1,Q,A\n");
        apply(&app, &preview(&app, &deck, &token, mapping(true)).unwrap()).unwrap();
        let card = app.library().unwrap().state.cards[0].id.clone();
        let token = source(&app, "id,q,a\n1,Q,Changed\n");
        let stale = preview(&app, &deck, &token, mapping(true)).unwrap();
        let command = match action {
            "grade" => Command::Grade {
                card_id: card,
                rating: 3,
            },
            "edit" => Command::EditCard {
                card_id: card,
                question: "Edited".into(),
                answer: "Keep".into(),
                explanation: String::new(),
                choices: vec![],
            },
            "bonus" => Command::AddBonus {
                deck_id: deck,
                new: 5,
                review: 0,
            },
            "configure" => Command::ConfigureDeck {
                deck_id: deck,
                retention: 0.8,
                new_limit: 10,
                review_limit: Some(30),
            },
            "settings" => Command::Settings {
                settings: Settings {
                    timezone: "UTC".into(),
                    day_start_hour: 5,
                },
            },
            "create" => Command::CreateDeck {
                name: "Other".into(),
            },
            "delete" => Command::DeleteDeck { deck_id: deck },
            "restore" => {
                let revision = app.library().unwrap().state.revision;
                *lock(&app.restore).unwrap() = Some(RestoreCandidate {
                    token: "restore".into(),
                    revision,
                    snapshot: Library::new("UTC").unwrap().state,
                });
                Command::Restore {
                    token: "restore".into(),
                }
            }
            _ => unreachable!(),
        };
        app.handle(command, 103).unwrap();
        let after = app.library().unwrap().state;
        assert!(apply(&app, &stale).is_err(), "action={action}");
        assert_eq!(app.library().unwrap().state, after, "action={action}");
    }
}

#[test]
fn preview_tokens_are_discarded_after_cancellation_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let app = AppState::new(directory.path().into()).unwrap();
    let deck = deck(&app, "Synthetic");
    let token = source(&app, "id,q,a\n1,Q,A\n");
    let candidate = preview(&app, &deck, &token, mapping(true)).unwrap();
    app.handle(Command::DiscardImport, 102).unwrap();
    assert!(apply(&app, &candidate).is_err());
    let token = source(&app, "id,q,a\n1,Q,A\n");
    let candidate = preview(&app, &deck, &token, mapping(true)).unwrap();
    drop(app);
    let app = AppState::new(directory.path().into()).unwrap();
    assert!(apply(&app, &candidate).is_err());
    assert!(app.library().unwrap().state.cards.is_empty());
}
