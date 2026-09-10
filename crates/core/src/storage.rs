use crate::*;
use rusqlite::{Connection, params};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct Database {
    conn: Connection,
    path: PathBuf,
}
impl Database {
    pub fn open(path: &Path, timezone: &str) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            // SQLite derives WAL/SHM permissions from the database at open.
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .mode(0o600)
                .open(path)?;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(invalid(
                "このアプリより新しいデータ形式です。アプリを手動更新してください。",
            ));
        }
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON; PRAGMA secure_delete=ON;")?;
        if version == 0 {
            let state = Library::new(timezone)?.state;
            conn.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE metadata (id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL, settings TEXT NOT NULL);
                CREATE TABLE decks (id TEXT PRIMARY KEY, position INTEGER NOT NULL, payload TEXT NOT NULL);
                CREATE TABLE cards (id TEXT PRIMARY KEY, position INTEGER NOT NULL, payload TEXT NOT NULL);
                CREATE TABLE reviews (id TEXT PRIMARY KEY, position INTEGER NOT NULL, payload TEXT NOT NULL);
                CREATE TABLE bonuses (id TEXT PRIMARY KEY, position INTEGER NOT NULL, payload TEXT NOT NULL);
                PRAGMA user_version=2;")?;
            conn.execute(
                "INSERT INTO metadata VALUES (1,0,?1)",
                [serde_json::to_string(&state.settings)?],
            )?;
            conn.execute_batch("COMMIT")?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
            for suffix in ["-wal", "-shm"] {
                let mut name = path.as_os_str().to_os_string();
                name.push(suffix);
                let sidecar = PathBuf::from(name);
                if sidecar.exists() {
                    std::fs::set_permissions(sidecar, std::fs::Permissions::from_mode(0o600))?;
                }
            }
        }
        let db = Self {
            conn,
            path: path.into(),
        };
        db.load()?;
        if version == 1 {
            // Version 1 payloads decode with empty choices. Mark the database only
            // after validating it, so older apps cannot overwrite choice data.
            db.conn
                .pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        Ok(db)
    }
    pub fn load(&self) -> Result<Snapshot> {
        let transaction = self.conn.unchecked_transaction()?;
        let (revision, settings): (i64, String) = transaction.query_row(
            "SELECT revision,settings FROM metadata WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let revision = u64::try_from(revision).map_err(|_| invalid("データの版が不正です。"))?;
        let state = Snapshot {
            schema: SCHEMA_VERSION,
            revision,
            settings: serde_json::from_str(&settings)?,
            decks: read_table(&transaction, "decks")?,
            cards: read_table(&transaction, "cards")?,
            reviews: read_table(&transaction, "reviews")?,
            bonuses: read_table(&transaction, "bonuses")?,
        };
        transaction.commit()?;
        validate_snapshot(&state)?;
        Ok(state)
    }
    pub fn save(&mut self, state: &Snapshot, expected: u64) -> Result<()> {
        validate_snapshot(state)?;
        if state.revision <= expected {
            return Err(invalid("保存するデータの版が不正です。"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current: i64 =
            tx.query_row("SELECT revision FROM metadata WHERE id=1", [], |r| r.get(0))?;
        if u64::try_from(current).ok() != Some(expected) {
            return Err(invalid(
                "別の操作でデータが更新されました。画面を更新してください。",
            ));
        }
        sync_table(&tx, "decks", &state.decks, |d| &d.id)?;
        sync_table(&tx, "cards", &state.cards, |c| &c.id)?;
        sync_table(&tx, "reviews", &state.reviews, |r| &r.id)?;
        sync_table(&tx, "bonuses", &state.bonuses, |b| &b.id)?;
        tx.execute(
            "UPDATE metadata SET revision=?1, settings=?2 WHERE id=1",
            params![
                state.revision as i64,
                serde_json::to_string(&state.settings)?
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn restore(&mut self, state: &Snapshot, expected: u64) -> Result<PathBuf> {
        validate_snapshot(state)?;
        if self.load()?.revision != expected {
            return Err(invalid(
                "プレビュー後にデータが変わりました。もう一度復元を確認してください。",
            ));
        }
        let dir = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("restore-safety");
        std::fs::create_dir_all(&dir)?;
        let safety = dir.join(format!("before-{}.sqlite3", uuid::Uuid::new_v4()));
        self.conn.backup("main", &safety, None)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&safety, std::fs::Permissions::from_mode(0o600))?;
        }
        let copy =
            Connection::open_with_flags(&safety, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let integrity: String = copy.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if integrity != "ok" {
            return Err(invalid(
                "復元前の退避データを確認できません。復元を中止しました。",
            ));
        }
        drop(copy);
        let mut state = state.clone();
        state.revision = expected
            .checked_add(1)
            .ok_or_else(|| invalid("データの版が範囲外です。"))?;
        self.save(&state, expected)?;
        Ok(safety)
    }
}

fn read_table<T: DeserializeOwned>(conn: &Connection, table: &str) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(&format!("SELECT payload FROM {table} ORDER BY position"))?;
    stmt.query_map([], |r| r.get::<_, String>(0))?
        .map(|r| Ok(serde_json::from_str(&r?)?))
        .collect()
}

fn sync_table<T: Serialize>(
    conn: &Connection,
    table: &str,
    items: &[T],
    id: impl Fn(&T) -> &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("SELECT id,position,payload FROM {table}"))?;
    let mut old = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                (r.get::<_, i64>(1)?, r.get::<_, String>(2)?),
            ))
        })?
        .collect::<std::result::Result<HashMap<_, _>, _>>()?;
    let mut upsert = conn.prepare(&format!("INSERT INTO {table}(id,position,payload) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET position=excluded.position,payload=excluded.payload"))?;
    for (position, item) in items.iter().enumerate() {
        let payload = serde_json::to_string(item)?;
        if old.remove(id(item)) != Some((position as i64, payload.clone())) {
            upsert.execute(params![id(item), position as i64, payload])?;
        }
    }
    let mut delete = conn.prepare(&format!("DELETE FROM {table} WHERE id=?1"))?;
    for key in old.keys() {
        delete.execute([key])?;
    }
    Ok(())
}

pub fn validate_snapshot(state: &Snapshot) -> Result<()> {
    if state.schema != SCHEMA_VERSION || state.revision > i64::MAX as u64 {
        return Err(invalid("対応していないバックアップ形式です。"));
    }
    study_day(0, &state.settings.timezone, state.settings.day_start_hour)?;
    let mut ids = HashSet::new();
    for id in state
        .decks
        .iter()
        .map(|d| &d.id)
        .chain(state.cards.iter().map(|c| &c.id))
        .chain(state.reviews.iter().map(|r| &r.id))
        .chain(state.bonuses.iter().map(|b| &b.id))
    {
        if uuid::Uuid::parse_str(id).is_err() || !ids.insert(id) {
            return Err(invalid("データの識別子が不正または重複しています。"));
        }
    }
    let decks: HashSet<_> = state.decks.iter().map(|d| &d.id).collect();
    for deck in &state.decks {
        validate_timestamp(deck.created_at)?;
        if deck.name.trim().is_empty()
            || deck.name.chars().count() > 200
            || !deck.retention.is_finite()
            || deck.retention <= 0.0
            || deck.retention >= 1.0
            || deck.parameter_version != ALGORITHM
        {
            return Err(invalid("単語帳の設定が不正または非対応です。"));
        }
        validate_parameters(&deck.parameters)?;
    }
    let cards: HashMap<_, _> = state.cards.iter().map(|c| (&c.id, c)).collect();
    for card in &state.cards {
        validate_timestamp(card.created_at)?;
        if !decks.contains(&card.deck_id)
            || card.question.trim().is_empty()
            || card.answer.trim().is_empty()
            || card.source_id.as_ref().is_some_and(|s| s.trim().is_empty())
            || card
                .choices
                .iter()
                .any(|choice| choice.text.trim().is_empty())
        {
            return Err(invalid("カードの内容または単語帳との関係が不正です。"));
        }
        validate_schedule(&card.schedule)?;
    }
    let mut latest: HashMap<&String, &Schedule> = HashMap::new();
    let mut last_at = None;
    for review in &state.reviews {
        validate_timestamp(review.at)?;
        let card = cards
            .get(&review.card_id)
            .ok_or_else(|| invalid("学習履歴に対応するカードがありません。"))?;
        if card.deck_id != review.deck_id
            || !(1..=4).contains(&review.rating)
            || review.algorithm != ALGORITHM
            || !["new", "review", "repeat"].contains(&review.kind.as_str())
            || last_at.is_some_and(|at| at > review.at)
        {
            return Err(invalid("学習履歴の内容が不正または非対応です。"));
        }
        chrono::NaiveDate::parse_from_str(&review.study_day, "%Y-%m-%d")
            .map_err(|_| invalid("学習日の形式が不正です。"))?;
        validate_parameters(&review.parameters)?;
        validate_schedule(&review.before)?;
        validate_schedule(&review.after)?;
        if latest
            .get(&review.card_id)
            .copied()
            .cloned()
            .unwrap_or_default()
            != review.before
            || review.after.reps
                != review
                    .before
                    .reps
                    .checked_add(1)
                    .ok_or_else(|| invalid("学習回数が範囲外です。"))?
            || review.after.last_review != Some(review.at)
        {
            return Err(invalid("学習履歴の連続性を確認できません。"));
        }
        latest.insert(&review.card_id, &review.after);
        last_at = Some(review.at);
    }
    for card in &state.cards {
        if latest.get(&card.id).copied().cloned().unwrap_or_default() != card.schedule {
            return Err(invalid("学習履歴とカードの状態が一致しません。"));
        }
    }
    let mut days = HashSet::new();
    for bonus in &state.bonuses {
        if !decks.contains(&bonus.deck_id) || !days.insert((&bonus.deck_id, &bonus.day)) {
            return Err(invalid("当日上乗せの単語帳または学習日が不正です。"));
        }
        let deck = state
            .decks
            .iter()
            .find(|d| d.id == bonus.deck_id)
            .ok_or_else(|| invalid("単語帳がありません。"))?;
        if deck.new_limit.checked_add(bonus.new).is_none()
            || deck
                .review_limit
                .is_some_and(|n| n.checked_add(bonus.review).is_none())
        {
            return Err(invalid("当日上乗せと合わせた枚数が大きすぎます。"));
        }
        chrono::NaiveDate::parse_from_str(&bonus.day, "%Y-%m-%d")
            .map_err(|_| invalid("学習日の形式が不正です。"))?;
    }
    Ok(())
}

fn validate_parameters(parameters: &[f32]) -> Result<()> {
    if parameters.len() != 21 || parameters.iter().any(|p| !p.is_finite()) || parameters[20] <= 0.0
    {
        return Err(invalid("FSRSパラメータが不正です。"));
    }
    fsrs::FSRS::new(parameters)?;
    Ok(())
}
fn validate_schedule(s: &Schedule) -> Result<()> {
    for at in [s.last_review, s.due].into_iter().flatten() {
        validate_timestamp(at)?;
    }
    if s.reps == 0 {
        if s != &Schedule::default() {
            return Err(invalid("未学習カードの状態が不正です。"));
        }
    } else if !s.stability.is_some_and(|v| v.is_finite() && v > 0.0)
        || !s
            .difficulty
            .is_some_and(|v| v.is_finite() && (1.0..=10.0).contains(&v))
        || s.last_review.is_none()
        || s.due <= s.last_review
        || s.lapses > s.reps
    {
        return Err(invalid("記憶状態または復習予定が不正です。"));
    }
    Ok(())
}

fn validate_timestamp(at: i64) -> Result<()> {
    // A deliberately portable calendar range, also safe for JavaScript dates.
    if !(-62_135_596_800..=253_402_300_799).contains(&at) {
        return Err(invalid("日時が対応範囲外です。"));
    }
    Ok(())
}
