use kotoba_core::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Default)]
pub struct ImportCache {
    pub source: Option<(String, ParsedCsv)>,
    pub preview: Option<(String, ImportPreview)>,
}
pub struct RestoreCandidate {
    pub token: String,
    pub revision: u64,
    pub snapshot: Snapshot,
}
struct OptimizationJob {
    deck: Option<String>,
    revision: u64,
    status: String,
    message: String,
    parameters: Option<Vec<f32>>,
    progress: Arc<Mutex<fsrs::CombinedProgressState>>,
}
impl Default for OptimizationJob {
    fn default() -> Self {
        Self {
            deck: None,
            revision: 0,
            status: "idle".into(),
            message: String::new(),
            parameters: None,
            progress: fsrs::CombinedProgressState::new_shared(),
        }
    }
}
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub imports: Arc<Mutex<ImportCache>>,
    pub restore: Arc<Mutex<Option<RestoreCandidate>>>,
    job: Arc<Mutex<OptimizationJob>>,
    pub directory: Arc<PathBuf>,
}
pub fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    value
        .lock()
        .map_err(|_| invalid("操作状態を読み取れません。アプリを再起動してください。"))
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    View,
    CreateDeck {
        name: String,
    },
    PreviewImport {
        source_token: String,
        deck_id: String,
        mapping: Mapping,
    },
    ApplyImport {
        token: String,
    },
    DiscardImport,
    Grade {
        card_id: String,
        rating: u32,
    },
    Undo,
    ConfigureDeck {
        deck_id: String,
        retention: f32,
        new_limit: u32,
        review_limit: Option<u32>,
    },
    AddBonus {
        deck_id: String,
        new: u32,
        review: u32,
    },
    Settings {
        settings: Settings,
    },
    EditCard {
        card_id: String,
        question: String,
        answer: String,
        explanation: String,
    },
    Analytics {
        deck_id: String,
    },
    Curve {
        card_id: String,
    },
    StartOptimization {
        deck_id: String,
    },
    Optimization,
    CancelOptimization,
    ApplyOptimization,
    Restore {
        token: String,
    },
    DiscardRestore,
}

impl AppState {
    pub fn new(directory: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
        }
        let timezone = iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".into());
        let db = Database::open(&directory.join("library.sqlite3"), &timezone)?;
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            imports: Default::default(),
            restore: Default::default(),
            job: Default::default(),
            directory: Arc::new(directory),
        })
    }
    pub fn library(&self) -> Result<Library> {
        Ok(Library {
            state: lock(&self.db)?.load()?,
        })
    }
    fn mutate(&self, operation: impl FnOnce(&mut Library) -> Result<Value>) -> Result<Value> {
        let mut db = lock(&self.db)?;
        let mut library = Library { state: db.load()? };
        let revision = library.state.revision;
        let output = operation(&mut library)?;
        db.save(&library.state, revision)?;
        Ok(output)
    }
    pub fn handle(&self, command: Command, now: i64) -> Result<Value> {
        match command {
            Command::View => {
                let lib = self.library()?;
                let queues: BTreeMap<_, _> = lib
                    .state
                    .decks
                    .iter()
                    .map(|d| Ok((d.id.clone(), lib.queue(&d.id, now)?)))
                    .collect::<Result<_>>()?;
                Ok(
                    json!({ "revision": lib.state.revision, "settings": lib.state.settings, "decks": lib.state.decks, "cards": lib.state.cards,
                    "queues": queues, "canUndo": !lib.state.reviews.is_empty(), "reviewCount": lib.state.reviews.len() }),
                )
            }
            Command::CreateDeck { name } => {
                self.mutate(|lib| Ok(json!(lib.create_deck(&name, now)?)))
            }
            Command::PreviewImport {
                source_token,
                deck_id,
                mapping,
            } => {
                let lib = self.library()?;
                let mut cache = lock(&self.imports)?;
                let (token, csv) = cache
                    .source
                    .as_ref()
                    .ok_or_else(|| invalid("CSVを選択してください。"))?;
                if *token != source_token {
                    return Err(invalid("CSVの選択が変わりました。選び直してください。"));
                }
                let preview = lib.preview_import(&deck_id, csv, mapping, now)?;
                let token = uuid::Uuid::new_v4().to_string();
                let output = json!({ "token": token, "deckId": preview.deck_id, "changes": preview.changes, "errors": preview.errors, "missing": preview.missing });
                cache.preview = Some((token, preview));
                Ok(output)
            }
            Command::ApplyImport { token } => {
                let preview = {
                    let cache = lock(&self.imports)?;
                    if cache
                        .preview
                        .as_ref()
                        .is_none_or(|(saved, _)| *saved != token)
                    {
                        return Err(invalid("更新内容をもう一度プレビューしてください。"));
                    }
                    cache
                        .preview
                        .as_ref()
                        .ok_or_else(|| invalid("プレビューがありません。"))?
                        .1
                        .clone()
                };
                self.mutate(|lib| {
                    lib.apply_import(preview)?;
                    Ok(Value::Null)
                })?;
                let mut cache = lock(&self.imports)?;
                if cache
                    .preview
                    .as_ref()
                    .is_some_and(|(saved, _)| *saved == token)
                {
                    *cache = ImportCache::default();
                }
                Ok(Value::Null)
            }
            Command::DiscardImport => {
                *lock(&self.imports)? = ImportCache::default();
                Ok(Value::Null)
            }
            Command::Grade { card_id, rating } => self.mutate(|lib| {
                lib.grade(&card_id, rating, now)?;
                Ok(Value::Null)
            }),
            Command::Undo => self.mutate(|lib| {
                lib.undo()?;
                Ok(Value::Null)
            }),
            Command::ConfigureDeck {
                deck_id,
                retention,
                new_limit,
                review_limit,
            } => self.mutate(|lib| {
                lib.configure_deck(&deck_id, retention, new_limit, review_limit)?;
                Ok(Value::Null)
            }),
            Command::AddBonus {
                deck_id,
                new,
                review,
            } => self.mutate(|lib| {
                lib.add_bonus(&deck_id, new, review, now)?;
                Ok(Value::Null)
            }),
            Command::Settings { settings } => self.mutate(|lib| {
                lib.set_settings(settings)?;
                Ok(Value::Null)
            }),
            Command::EditCard {
                card_id,
                question,
                answer,
                explanation,
            } => self.mutate(|lib| {
                lib.edit_card(&card_id, &question, &answer, &explanation)?;
                Ok(Value::Null)
            }),
            Command::Analytics { deck_id } => Ok(serde_json::to_value(
                self.library()?.analytics(&deck_id, now)?,
            )?),
            Command::Curve { card_id } => {
                Ok(serde_json::to_value(self.library()?.curve(&card_id, now)?)?)
            }
            Command::StartOptimization { deck_id } => {
                let lib = self.library()?;
                lib.deck(&deck_id)?;
                let mut job = lock(&self.job)?;
                if job.status == "running" {
                    return Err(invalid(
                        "現在の計算が終わるか、中断してから再実行してください。",
                    ));
                }
                *job = OptimizationJob {
                    deck: Some(deck_id.clone()),
                    revision: lib.state.revision,
                    status: "running".into(),
                    message: "端末内で計算しています。".into(),
                    ..Default::default()
                };
                let progress = job.progress.clone();
                let job_state = self.job.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let result = lib.optimize(&deck_id, progress.clone());
                    if let Ok(mut job) = job_state.lock() {
                        if progress.lock().map(|p| p.want_abort).unwrap_or(true) {
                            job.status = "cancelled".into();
                            job.message = "計算を中断しました。設定は変更していません。".into();
                        } else {
                            match result {
                                Ok(parameters) => {
                                    job.parameters = Some(parameters);
                                    job.status = "ready".into();
                                    job.message =
                                        "計算が完了しました。適用すると今後の復習に使用します。"
                                            .into();
                                }
                                Err(error) => {
                                    job.status = "failed".into();
                                    job.message = error.to_string();
                                }
                            }
                        }
                    }
                });
                Ok(Value::Null)
            }
            Command::Optimization => {
                let job = lock(&self.job)?;
                let progress = lock(&job.progress)?;
                Ok(
                    json!({ "status": job.status, "current": progress.current(), "total": progress.total(), "message": job.message, "deckId": job.deck }),
                )
            }
            Command::CancelOptimization => {
                let mut job = lock(&self.job)?;
                lock(&job.progress)?.want_abort = true;
                if job.status == "ready" {
                    job.parameters = None;
                    job.status = "cancelled".into();
                    job.message = "計算結果を破棄しました。設定は変更していません。".into();
                }
                Ok(Value::Null)
            }
            Command::ApplyOptimization => {
                let mut job = lock(&self.job)?;
                let (deck, revision, parameters) = {
                    if job.status != "ready" || lock(&job.progress)?.want_abort {
                        return Err(invalid("適用できる計算結果がありません。"));
                    }
                    (
                        job.deck
                            .clone()
                            .ok_or_else(|| invalid("単語帳がありません。"))?,
                        job.revision,
                        job.parameters
                            .clone()
                            .ok_or_else(|| invalid("計算結果がありません。"))?,
                    )
                };
                self.mutate(|lib| {
                    lib.apply_parameters(&deck, parameters, revision)?;
                    Ok(Value::Null)
                })?;
                job.status = "applied".into();
                job.message = "今後の復習に使う設定を更新しました。".into();
                job.parameters = None;
                Ok(Value::Null)
            }
            Command::Restore { token } => {
                let candidate = {
                    let mut restore = lock(&self.restore)?;
                    if restore.as_ref().is_none_or(|p| p.token != token) {
                        return Err(invalid("復元内容をもう一度確認してください。"));
                    }
                    restore
                        .take()
                        .ok_or_else(|| invalid("復元内容がありません。"))?
                };
                lock(&self.db)?.restore(&candidate.snapshot, candidate.revision)?;
                *lock(&self.imports)? = ImportCache::default();
                Ok(Value::Null)
            }
            Command::DiscardRestore => {
                *lock(&self.restore)? = None;
                Ok(Value::Null)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_still_discards_a_result_that_just_finished() {
        let directory = tempfile::tempdir().unwrap();
        let app = AppState::new(directory.path().into()).unwrap();
        let deck = app
            .handle(
                Command::CreateDeck {
                    name: "Synthetic".into(),
                },
                100,
            )
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let before = app.library().unwrap().state;
        *lock(&app.job).unwrap() = OptimizationJob {
            deck: Some(deck),
            revision: before.revision,
            status: "ready".into(),
            parameters: Some(fsrs::DEFAULT_PARAMETERS.to_vec()),
            ..Default::default()
        };
        app.handle(Command::CancelOptimization, 100).unwrap();
        assert!(app.handle(Command::ApplyOptimization, 100).is_err());
        assert_eq!(app.library().unwrap().state, before);
        assert_eq!(
            app.handle(Command::Optimization, 100).unwrap()["status"],
            "cancelled"
        );
    }

    #[test]
    fn stale_preview_can_be_regenerated_without_reselecting_the_csv() {
        let directory = tempfile::tempdir().unwrap();
        let app = AppState::new(directory.path().into()).unwrap();
        let id = app
            .handle(
                Command::CreateDeck {
                    name: "Synthetic".into(),
                },
                100,
            )
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        let csv = parse_csv(b"q,a\nx,y\n", "utf-8").unwrap();
        lock(&app.imports).unwrap().source = Some(("source".into(), csv));
        let request = || Command::PreviewImport {
            source_token: "source".into(),
            deck_id: id.clone(),
            mapping: Mapping {
                question: 0,
                answer: 1,
                explanation: None,
                id: None,
            },
        };
        let preview = app.handle(request(), 100).unwrap();
        app.handle(
            Command::CreateDeck {
                name: "Another".into(),
            },
            101,
        )
        .unwrap();
        assert!(
            app.handle(
                Command::ApplyImport {
                    token: preview["token"].as_str().unwrap().into()
                },
                102
            )
            .is_err()
        );
        assert!(app.library().unwrap().state.cards.is_empty());
        assert!(app.handle(request(), 103).is_ok());
    }
}
