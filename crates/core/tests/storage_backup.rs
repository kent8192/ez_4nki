use kotoba_core::*;

fn populated() -> Snapshot {
    let mut lib = Library::new("Asia/Tokyo").unwrap();
    let id = lib
        .create_deck("架空の検証用単語帳", 1_789_000_000)
        .unwrap();
    let csv = parse_csv(b"q,a,options\n2+3,5,\"A. 4\nB. 5\"\n", "utf-8").unwrap();
    lib.apply_import(
        lib.preview_import(
            &id,
            &csv,
            Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
                choices: vec![2],
                choice_separator: Some("\n".into()),
            },
            1_789_000_000,
        )
        .unwrap(),
    )
    .unwrap();
    let id = lib.state.cards[0].id.clone();
    lib.grade(&id, 3, 1_789_000_000).unwrap();
    lib.state
}

#[test]
fn sqlite_reopen_preserves_history_and_refuses_stale_writer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Database::open(&path, "Asia/Tokyo").unwrap();
    let state = populated();
    db.save(&state, 0).unwrap();
    drop(db);
    let mut db = Database::open(&path, "UTC").unwrap();
    assert_eq!(db.load().unwrap(), state);
    assert!(db.save(&Library::new("UTC").unwrap().state, 0).is_err());
    assert_eq!(db.load().unwrap(), state);
}

#[test]
fn encrypted_backup_round_trips_and_rejects_wrong_password_or_truncation() {
    let state = populated();
    let password = "synthetic test passphrase only";
    let bytes = encode_backup(&state, password.into()).unwrap();
    assert!(bytes.starts_with(b"age-encryption.org/v1"));
    assert!(
        !bytes
            .windows("架空の検証用単語帳".len())
            .any(|w| w == "架空の検証用単語帳".as_bytes())
    );
    assert_eq!(decode_backup(&bytes, password.into()).unwrap(), state);
    assert!(decode_backup(&bytes, "wrong synthetic passphrase".into()).is_err());
    assert!(decode_backup(&bytes[..bytes.len() - 4], password.into()).is_err());
}

#[test]
fn restore_replaces_all_data_and_retains_a_readable_local_safety_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Database::open(&path, "UTC").unwrap();
    let old = db.load().unwrap();
    let incoming = populated();
    let safety_path = db.restore(&incoming, old.revision).unwrap();
    let restored = db.load().unwrap();
    assert_eq!(restored.cards, incoming.cards);
    assert_eq!(restored.reviews, incoming.reviews);
    assert_eq!(restored.settings, incoming.settings);
    assert_eq!(
        Database::open(&safety_path, "UTC").unwrap().load().unwrap(),
        old
    );
    let mut invalid = incoming.clone();
    invalid.schema = 999;
    assert!(db.restore(&invalid, restored.revision).is_err());
    assert_eq!(db.load().unwrap(), restored);
}

#[test]
fn invalid_relationships_and_impossible_memory_are_rejected_before_saving() {
    let state = populated();
    assert!(validate_snapshot(&state).is_ok());
    let mut broken = state.clone();
    broken.cards[0].deck_id = "nonexistent".into();
    assert!(validate_snapshot(&broken).is_err());
    let mut broken = state.clone();
    broken.cards[0].schedule.stability = Some(f32::NAN);
    assert!(validate_snapshot(&broken).is_err());
    let mut broken = state.clone();
    broken.reviews.clear();
    assert!(validate_snapshot(&broken).is_err());
    let mut broken = state.clone();
    broken.cards.push(broken.cards[0].clone());
    assert!(validate_snapshot(&broken).is_err());
    let mut broken = state.clone();
    broken.cards[0].choices[0].text = "   ".into();
    assert!(validate_snapshot(&broken).is_err());
}

fn legacy_json() -> serde_json::Value {
    let mut value = serde_json::to_value(populated()).unwrap();
    value["schema"] = 1.into();
    for card in value["cards"].as_array_mut().unwrap() {
        card.as_object_mut().unwrap().remove("choices");
    }
    for deck in value["decks"].as_array_mut().unwrap() {
        let mapping = deck["mapping"].as_object_mut().unwrap();
        mapping.remove("choices");
        mapping.remove("choiceSeparator");
    }
    value
}

#[test]
fn version_one_database_upgrades_without_changing_cards_or_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let value = legacy_json();
    let legacy: Snapshot = serde_json::from_value(value.clone()).unwrap();
    let mut expected = legacy.clone();
    expected.schema = SCHEMA_VERSION;
    let mut db = Database::open(&path, "UTC").unwrap();
    db.save(&expected, 0).unwrap();
    drop(db);
    let conn = rusqlite::Connection::open(&path).unwrap();
    for (table, items) in [("cards", &value["cards"]), ("decks", &value["decks"])] {
        for item in items.as_array().unwrap() {
            conn.execute(
                &format!("UPDATE {table} SET payload=?1 WHERE id=?2"),
                rusqlite::params![item.to_string(), item["id"].as_str().unwrap()],
            )
            .unwrap();
        }
    }
    conn.pragma_update(None, "user_version", 1).unwrap();
    drop(conn);
    let db = Database::open(&path, "Asia/Tokyo").unwrap();
    assert_eq!(db.load().unwrap(), expected);
    drop(db);
    let conn = rusqlite::Connection::open(&path).unwrap();
    let version: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
    let payload: String = conn
        .query_row("SELECT payload FROM cards LIMIT 1", [], |row| row.get(0))
        .unwrap();
    assert!(!payload.contains("choices"));
}

#[test]
fn version_one_encrypted_backup_restores_with_empty_choices_and_the_same_history() {
    use std::io::Write;
    let value = legacy_json();
    let password = "synthetic legacy fixture only";
    let encryptor = age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(
        password.to_string(),
    ));
    let mut bytes = vec![];
    let mut writer = encryptor.wrap_output(&mut bytes).unwrap();
    writer
        .write_all(&serde_json::to_vec(&value).unwrap())
        .unwrap();
    writer.finish().unwrap();
    let restored = decode_backup(&bytes, password.into()).unwrap();
    let mut expected: Snapshot = serde_json::from_value(value).unwrap();
    expected.schema = SCHEMA_VERSION;
    assert_eq!(restored, expected);
    assert!(restored.cards[0].choices.is_empty());
    assert!(
        restored.decks[0]
            .mapping
            .as_ref()
            .unwrap()
            .choices
            .is_empty()
    );
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(&dir.path().join("restore.sqlite3"), "UTC").unwrap();
    db.restore(&restored, 0).unwrap();
    assert_eq!(db.load().unwrap().reviews, expected.reviews);
}

#[test]
fn failed_safety_copy_keeps_the_current_database_intact() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(&dir.path().join("library.sqlite3"), "UTC").unwrap();
    let before = db.load().unwrap();
    std::fs::write(dir.path().join("restore-safety"), "synthetic blocker").unwrap();
    assert!(db.restore(&populated(), before.revision).is_err());
    assert_eq!(db.load().unwrap(), before);
}

#[test]
fn backup_validation_rejects_out_of_range_timestamps_and_overflowing_limits() {
    let state = populated();
    let mut broken = state.clone();
    broken.cards[0].created_at = i64::MAX;
    assert!(validate_snapshot(&broken).is_err());
    let mut broken = state.clone();
    broken.cards[0].schedule.due = Some(i64::MAX);
    broken.reviews[0].after.due = Some(i64::MAX);
    assert!(validate_snapshot(&broken).is_err());
    let mut lib = Library { state };
    let deck = lib.state.decks[0].id.clone();
    lib.add_bonus(&deck, 1, 0, 1_789_000_000).unwrap();
    let before = lib.state.clone();
    assert!(lib.configure_deck(&deck, 0.9, u32::MAX, None).is_err());
    assert_eq!(lib.state, before);
    lib.state.decks[0].new_limit = u32::MAX;
    assert!(validate_snapshot(&lib.state).is_err());
}

#[cfg(unix)]
#[test]
fn database_and_wal_are_private_from_their_creation() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Database::open(&path, "UTC").unwrap();
    db.save(&populated(), 0).unwrap();
    for name in [
        "library.sqlite3",
        "library.sqlite3-wal",
        "library.sqlite3-shm",
    ] {
        assert_eq!(
            std::fs::metadata(dir.path().join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600,
            "{name}"
        );
    }
}
