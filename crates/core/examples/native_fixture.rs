//! Synthetic state for native UI tests on an isolated CI runner.
use kotoba_core::*;
use std::path::PathBuf;

fn main() {
    assert_eq!(std::env::var("CI").as_deref(), Ok("true"));
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "Usage: native_fixture seed|check DIRECTORY");
    let directory = PathBuf::from(&args[2]);
    let path = directory.join("library.sqlite3");
    match args[1].as_str() {
        "seed" => {
            // Refuse existing state, including an empty application's data directory.
            assert!(
                !directory.exists(),
                "Refusing to overwrite an existing directory"
            );
            let mut lib = Library::new("Asia/Tokyo").unwrap();
            let now = chrono::Utc::now().timestamp();
            let deck = lib
                .create_deck("Synthetic native verification", now)
                .unwrap();
            let csv = parse_csv(
                b"id,q,a,e\n001,What is 2 + 3?,5 <img src=x>,Synthetic plain text\n002,Second question,Second answer,Synthetic\n",
                "utf-8",
            ).unwrap();
            let preview = lib
                .preview_import(
                    &deck,
                    &csv,
                    Mapping {
                        question: 1,
                        answer: 2,
                        explanation: Some(3),
                        id: Some(0),
                    },
                    now,
                )
                .unwrap();
            lib.apply_import(preview).unwrap();
            Database::open(&path, "UTC")
                .unwrap()
                .save(&lib.state, 0)
                .unwrap();
        }
        "check" => {
            assert!(path.is_file());
            let state = Database::open(&path, "UTC").unwrap().load().unwrap();
            assert_eq!(state.decks.len(), 1);
            assert_eq!(state.cards.len(), 2);
            assert_eq!(state.reviews.len(), 1);
            assert_eq!(state.reviews[0].rating, 3);
            assert_eq!(state.cards[0].schedule.reps, 1);
            assert_eq!(state.cards[1].schedule.reps, 0);
            assert_eq!(state.cards[0].answer, "5 <img src=x>");
            assert_eq!(state.decks[0].new_limit, 20);
            assert_eq!(state.bonuses.len(), 1);
            assert_eq!(state.bonuses[0].new, 10);
            assert_eq!(state.settings.day_start_hour, 4);
            println!(
                "Native UI changes persisted: one grade, intact text, temporary bonus, unchanged permanent limit"
            );
        }
        _ => panic!("Only synthetic seed/check operations are supported"),
    }
}
