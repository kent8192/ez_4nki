//! Synthetic interoperability probe. Never imports application/user data.
use kotoba_core::*;
use std::path::PathBuf;

const PASSPHRASE: &str = "public synthetic interoperability fixture";
fn fixture() -> Snapshot {
    let mut lib = Library::new("Asia/Tokyo").unwrap();
    let now = 1_789_000_000;
    let deck = lib.create_deck("合成データ・相互復元", now).unwrap();
    let csv = parse_csv(
        "ID,問題,答え,解説,選択肢\n001,2+3は？,5,架空の検証,\"A. 4\nB. 5\"\n002,Hello,こんにちは,挨拶,\n".as_bytes(),
        "utf-8",
    )
    .unwrap();
    lib.apply_import(
        lib.preview_import(
            &deck,
            &csv,
            Mapping {
                question: 1,
                answer: 2,
                explanation: Some(3),
                id: Some(0),
                choices: vec![4],
                choice_separator: Some("\n".into()),
            },
            now,
        )
        .unwrap(),
    )
    .unwrap();
    lib.grade(&lib.state.cards[0].id.clone(), 3, now).unwrap();
    lib.add_bonus(&deck, 10, 0, now).unwrap();
    lib.state
}
fn verify(state: &Snapshot) {
    validate_snapshot(state).unwrap();
    assert_eq!(state.decks.len(), 1);
    assert_eq!(state.cards.len(), 2);
    assert_eq!(state.cards[0].question, "2+3は？");
    assert_eq!(
        state.cards[0]
            .choices
            .iter()
            .map(|choice| choice.text.as_str())
            .collect::<Vec<_>>(),
        vec!["A. 4", "B. 5"]
    );
    assert_eq!(state.reviews.len(), 1);
    assert_eq!(state.settings.timezone, "Asia/Tokyo");
    assert_eq!(state.settings.day_start_hour, 4);
    assert_eq!(state.bonuses[0].new, 10);
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        3,
        "Usage: portability_fixture create|check PATH.age"
    );
    let path = PathBuf::from(&args[2]);
    match args[1].as_str() {
        "create" => {
            let bytes = encode_backup(&fixture(), PASSPHRASE.into()).unwrap();
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();
            file.write_all(&bytes).unwrap();
        }
        "check" => {
            let state = decode_backup(&std::fs::read(path).unwrap(), PASSPHRASE.into()).unwrap();
            verify(&state);
            let temp = tempfile::tempdir().unwrap();
            let db_path = temp.path().join("library.sqlite3");
            let mut db = Database::open(&db_path, "UTC").unwrap();
            db.restore(&state, 0).unwrap();
            drop(db);
            let restored = Database::open(&db_path, "UTC").unwrap().load().unwrap();
            assert_eq!(restored.cards, state.cards);
            assert_eq!(restored.reviews, state.reviews);
            verify(&restored);
        }
        _ => panic!("Only synthetic create/check operations are supported"),
    }
    println!(
        "Synthetic backup probe passed on {} / {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}
