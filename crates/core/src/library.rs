use crate::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct Library {
    pub state: Snapshot,
}

struct ImportGroup {
    key: String,
    question: String,
    source_id: Option<String>,
    answers: Vec<String>,
    explanations: Vec<String>,
    choices: Vec<Choice>,
    lines: Vec<usize>,
}

fn append_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

impl Library {
    pub fn new(timezone: &str) -> Result<Self> {
        study_day(0, timezone, 4)?;
        Ok(Self {
            state: Snapshot {
                schema: SCHEMA_VERSION,
                revision: 0,
                settings: Settings {
                    timezone: timezone.into(),
                    day_start_hour: 4,
                },
                decks: vec![],
                cards: vec![],
                reviews: vec![],
                bonuses: vec![],
            },
        })
    }
    pub fn create_deck(&mut self, name: &str, now: i64) -> Result<String> {
        if name.trim().is_empty() || name.chars().count() > 200 {
            return Err(invalid("単語帳名を1〜200文字で入力してください。"));
        }
        let id = Uuid::new_v4().to_string();
        self.state.decks.push(Deck {
            id: id.clone(),
            name: name.trim().into(),
            created_at: now,
            retention: 0.9,
            new_limit: 20,
            review_limit: None,
            parameters: fsrs::DEFAULT_PARAMETERS.to_vec(),
            parameter_version: ALGORITHM.into(),
            mapping: None,
        });
        self.state.revision += 1;
        Ok(id)
    }
    pub fn delete_deck(&mut self, id: &str) -> Result<()> {
        self.deck(id)?;
        self.state.decks.retain(|deck| deck.id != id);
        self.state.cards.retain(|card| card.deck_id != id);
        self.state.reviews.retain(|review| review.deck_id != id);
        self.state.bonuses.retain(|bonus| bonus.deck_id != id);
        self.state.revision += 1;
        Ok(())
    }
    pub fn preview_import(
        &self,
        deck: &str,
        csv: &ParsedCsv,
        mapping: Mapping,
        now: i64,
    ) -> Result<ImportPreview> {
        if !self.state.decks.iter().any(|d| d.id == deck) {
            return Err(invalid("単語帳が見つかりません。"));
        }
        let columns: Vec<_> = [
            Some(mapping.question),
            Some(mapping.answer),
            mapping.explanation,
            mapping.id,
        ]
        .into_iter()
        .flatten()
        .chain(mapping.choices.iter().copied())
        .collect();
        if columns.iter().any(|i| *i >= csv.headers.len())
            || columns.iter().collect::<HashSet<_>>().len() != columns.len()
        {
            return Err(invalid(
                "列の割り当てを確認してください。同じ列は複数の役割に使えません。",
            ));
        }
        if !mapping.choices.is_empty() && mapping.choice_separator.as_deref() == Some("") {
            return Err(invalid("選択肢の区切り文字を入力してください。"));
        }
        let mut choice_columns = mapping.choices.clone();
        choice_columns.sort_unstable();
        let existing: Vec<_> = self
            .state
            .cards
            .iter()
            .filter(|c| c.deck_id == deck)
            .collect();
        let mut preview = ImportPreview {
            revision: self.state.revision,
            deck_id: deck.into(),
            mapping: mapping.clone(),
            changes: vec![],
            errors: vec![],
            missing: 0,
            merged_rows: 0,
        };
        let mut keys: HashMap<String, usize> = HashMap::new();
        let mut groups: Vec<ImportGroup> = vec![];
        for (index, row) in csv.rows.iter().enumerate() {
            let line = csv.row_lines.get(index).copied().unwrap_or(index + 2);
            if row.len() != csv.headers.len() {
                preview.errors.push(CsvIssue {
                    line,
                    message: "列数が一致しません。".into(),
                });
                continue;
            }
            let question = row[mapping.question].clone();
            let answer = row[mapping.answer].clone();
            let explanation = mapping
                .explanation
                .map(|i| row[i].clone())
                .unwrap_or_default();
            let source_id = mapping.id.map(|i| row[i].clone());
            if question.trim().is_empty()
                || answer.trim().is_empty()
                || source_id.as_ref().is_some_and(|id| id.trim().is_empty())
            {
                preview.errors.push(CsvIssue {
                    line,
                    message: "問題・答え・選択したIDは空欄にできません。".into(),
                });
                continue;
            }
            let key = source_id.as_deref().unwrap_or(&question);
            let group_index = if let Some(&group_index) = keys.get(key) {
                if groups[group_index].question != question {
                    preview.errors.push(CsvIssue {
                        line,
                        message: format!(
                            "{}行目と同じIDですが、問題文が異なります。問題文で照合する場合は「照合用ID」を「使用しない」にしてください。IDで照合する場合は、別の問題に別のIDを指定してください。",
                            groups[group_index].lines[0]
                        ),
                    });
                    continue;
                }
                group_index
            } else {
                let group_index = groups.len();
                keys.insert(key.to_string(), group_index);
                groups.push(ImportGroup {
                    key: key.to_string(),
                    question,
                    source_id,
                    answers: vec![],
                    explanations: vec![],
                    choices: vec![],
                    lines: vec![],
                });
                group_index
            };
            let group = &mut groups[group_index];
            group.lines.push(line);
            append_unique(&mut group.answers, answer);
            if !explanation.trim().is_empty() {
                append_unique(&mut group.explanations, explanation);
            }
            for &column in &choice_columns {
                let parts: Vec<_> = match mapping.choice_separator.as_deref() {
                    Some("\n") => row[column].lines().map(str::trim).collect(),
                    Some(separator) => row[column].split(separator).map(str::trim).collect(),
                    None => vec![row[column].as_str()],
                };
                for text in parts {
                    if !text.trim().is_empty() {
                        append_unique(
                            &mut group.choices,
                            Choice {
                                label: if mapping.choice_separator.is_some() {
                                    String::new()
                                } else {
                                    csv.headers[column].clone()
                                },
                                text: text.into(),
                            },
                        );
                    }
                }
            }
        }
        let mut matched = HashSet::new();
        for group in groups {
            let line = group.lines[0];
            let matches: Vec<_> = existing
                .iter()
                .filter(|c| {
                    if mapping.id.is_some() {
                        c.source_id.as_deref() == Some(&group.key)
                    } else {
                        c.question == group.key
                    }
                })
                .collect();
            if matches.len() > 1 {
                preview.errors.push(CsvIssue {
                    line,
                    message:
                        "既存カードの候補が複数あります。IDを指定するか、重複を解消してください。"
                            .into(),
                });
                continue;
            }
            let before = matches.first().map(|c| (***c).clone());
            let mut after = before.clone().unwrap_or_else(|| Card {
                id: Uuid::new_v4().to_string(),
                deck_id: deck.into(),
                source_id: None,
                question: String::new(),
                answer: String::new(),
                explanation: String::new(),
                choices: vec![],
                created_at: now,
                schedule: Schedule::default(),
            });
            if let Some(card) = &before {
                matched.insert(card.id.clone());
            }
            after.question = group.question;
            after.answer = group.answers.join("\n\n");
            after.explanation = group.explanations.join("\n\n");
            after.choices = group.choices;
            if mapping.id.is_some() {
                after.source_id = group.source_id;
            }
            let kind = if before.is_none() {
                "new"
            } else if before.as_ref() == Some(&after) {
                "unchanged"
            } else {
                "update"
            };
            let mut notes = vec![];
            for (field, count) in [
                ("答え", group.answers.len()),
                ("解説", group.explanations.len()),
            ] {
                if count > 1 {
                    notes.push(format!(
                        "異なる{field}を{count}件まとめています。内容を確認してください。"
                    ));
                }
            }
            preview.merged_rows += group.lines.len() - 1;
            preview.changes.push(Change {
                line,
                source_lines: group.lines,
                notes,
                kind: kind.into(),
                before,
                after,
            });
        }
        preview.missing = existing.len() - matched.len();
        Ok(preview)
    }
    pub fn apply_import(&mut self, preview: ImportPreview) -> Result<()> {
        if preview.revision != self.state.revision {
            return Err(invalid(
                "プレビュー後にデータが変わりました。もう一度プレビューしてください。",
            ));
        }
        if !preview.errors.is_empty() {
            return Err(invalid("CSVの要確認項目を解消してください。"));
        }
        let deck = self
            .state
            .decks
            .iter_mut()
            .find(|d| d.id == preview.deck_id)
            .ok_or_else(|| invalid("単語帳が見つかりません。"))?;
        deck.mapping = Some(preview.mapping);
        for change in preview.changes {
            if let Some(card) = self
                .state
                .cards
                .iter_mut()
                .find(|c| c.id == change.after.id)
            {
                *card = change.after;
            } else {
                self.state.cards.push(change.after);
            }
        }
        self.state.revision += 1;
        Ok(())
    }
}

pub fn parse_csv(bytes: &[u8], encoding: &str) -> Result<ParsedCsv> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(invalid("CSVは32 MB以内にしてください。"));
    }
    let text = match encoding {
        "utf-8" => std::str::from_utf8(bytes)
            .map_err(|_| invalid("UTF-8で読めません。文字コードを確認してください。"))?
            .to_string(),
        "cp932" => {
            let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(bytes);
            if errors {
                return Err(invalid("CP932で読めません。文字コードを確認してください。"));
            }
            text.into_owned()
        }
        _ => return Err(invalid("対応していない文字コードです。")),
    };
    let text = text.trim_start_matches('\u{feff}');
    validate_csv_quotes(text)?;
    let mut reader = csv::ReaderBuilder::new()
        .flexible(false)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| invalid("CSVの見出しを読み取れません。"))?
        .iter()
        .map(String::from)
        .collect::<Vec<_>>();
    if headers.len() < 2 {
        return Err(invalid("CSVには問題と答えの2列以上が必要です。"));
    }
    let mut rows = vec![];
    let mut row_lines = vec![];
    for row in reader.records() {
        let row = row.map_err(|e| {
            invalid(&format!(
                "CSVの{}行付近で列数または書式が一致しません。",
                e.position().map(|p| p.line()).unwrap_or(0)
            ))
        })?;
        row_lines.push(
            row.position()
                .map(|p| p.line() as usize)
                .unwrap_or(rows.len() + 2),
        );
        rows.push(row.iter().map(String::from).collect());
    }
    if rows.is_empty() {
        return Err(invalid("CSVにデータ行がありません。"));
    }
    Ok(ParsedCsv {
        headers,
        rows,
        row_lines,
    })
}

fn validate_csv_quotes(text: &str) -> Result<()> {
    let bytes = text.as_bytes();
    let (mut quoted, mut start, mut closed, mut line, mut i) = (false, true, false, 1, 0);
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' {
            line += 1;
        }
        if quoted {
            if b == b'"' {
                if bytes.get(i + 1) == Some(&b'"') {
                    i += 1;
                } else {
                    quoted = false;
                    closed = true;
                }
            }
        } else if matches!(b, b',' | b'\n' | b'\r') {
            start = true;
            closed = false;
        } else if b == b'"' && start {
            quoted = true;
            start = false;
        } else if b == b'"' || closed {
            return Err(invalid(&format!(
                "CSVの{line}行付近で引用符の書式が不正です。"
            )));
        } else {
            start = false;
        }
        i += 1;
    }
    if quoted {
        return Err(invalid(&format!(
            "CSVの{line}行付近で引用符が閉じられていません。"
        )));
    }
    Ok(())
}
