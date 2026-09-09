use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
pub const ALGORITHM: &str = "FSRS-6 / fsrs-rs 6.6.2";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("保存処理に失敗しました。現在のデータは変更されていません。")]
    Storage(#[from] rusqlite::Error),
    #[error("ファイルの読み書きに失敗しました。")]
    Io(#[from] std::io::Error),
    #[error("データ形式を読み取れません。")]
    Json(#[from] serde_json::Error),
    #[error("学習計算に失敗しました。")]
    Fsrs(#[from] fsrs::FSRSError),
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub timezone: String,
    pub day_start_hour: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mapping {
    pub question: usize,
    pub answer: usize,
    pub explanation: Option<usize>,
    pub id: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Deck {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub retention: f32,
    pub new_limit: u32,
    pub review_limit: Option<u32>,
    pub parameters: Vec<f32>,
    pub parameter_version: String,
    pub mapping: Option<Mapping>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Schedule {
    pub stability: Option<f32>,
    pub difficulty: Option<f32>,
    pub last_review: Option<i64>,
    pub due: Option<i64>,
    pub reps: u32,
    pub lapses: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Card {
    pub id: String,
    pub deck_id: String,
    pub source_id: Option<String>,
    pub question: String,
    pub answer: String,
    pub explanation: String,
    pub created_at: i64,
    pub schedule: Schedule,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Review {
    pub id: String,
    pub card_id: String,
    pub deck_id: String,
    pub at: i64,
    pub rating: u32,
    pub elapsed_days: u32,
    pub study_day: String,
    pub kind: String,
    pub before: Schedule,
    pub after: Schedule,
    pub parameters: Vec<f32>,
    pub algorithm: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bonus {
    pub id: String,
    pub deck_id: String,
    pub day: String,
    pub new: u32,
    pub review: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u32,
    pub revision: u64,
    pub settings: Settings,
    pub decks: Vec<Deck>,
    pub cards: Vec<Card>,
    pub reviews: Vec<Review>,
    pub bonuses: Vec<Bonus>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedCsv {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub row_lines: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub line: usize,
    pub kind: String,
    pub before: Option<Card>,
    pub after: Card,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvIssue {
    pub line: usize,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub revision: u64,
    pub deck_id: String,
    pub mapping: Mapping,
    pub changes: Vec<Change>,
    pub errors: Vec<CsvIssue>,
    pub missing: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub day: String,
    pub card_ids: Vec<String>,
    pub remaining_new: usize,
    pub due_reviews: usize,
    pub new_used: u32,
    pub review_used: u32,
    pub new_limit: u32,
    pub review_limit: Option<u32>,
    pub new_bonus: u32,
    pub review_bonus: u32,
}
