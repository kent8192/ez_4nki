use crate::*;
use std::collections::BTreeMap;
use uuid::Uuid;

impl Library {
    pub fn day(&self, now: i64) -> Result<String> {
        study_day(
            now,
            &self.state.settings.timezone,
            self.state.settings.day_start_hour,
        )
    }
    pub fn deck(&self, id: &str) -> Result<&Deck> {
        self.state
            .decks
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| invalid("単語帳が見つかりません。"))
    }
    fn reviewed_today(&self, deck: &str, now: i64) -> Result<BTreeMap<String, bool>> {
        let day = self.day(now)?;
        let mut seen = BTreeMap::new();
        for review in self.state.reviews.iter().filter(|r| r.deck_id == deck) {
            if self.day(review.at)? == day {
                seen.entry(review.card_id.clone())
                    .or_insert(review.before.reps == 0);
            }
        }
        Ok(seen)
    }
    pub fn queue(&self, deck: &str, now: i64) -> Result<Queue> {
        let config = self.deck(deck)?;
        let day = self.day(now)?;
        let bonus = self
            .state
            .bonuses
            .iter()
            .find(|b| b.deck_id == deck && b.day == day);
        let new_bonus = bonus.map(|b| b.new).unwrap_or(0);
        let review_bonus = bonus.map(|b| b.review).unwrap_or(0);
        let new_limit = config
            .new_limit
            .checked_add(new_bonus)
            .ok_or_else(|| invalid("枚数が大きすぎます。"))?;
        let review_limit = config
            .review_limit
            .map(|n| {
                n.checked_add(review_bonus)
                    .ok_or_else(|| invalid("枚数が大きすぎます。"))
            })
            .transpose()?;
        let seen = self.reviewed_today(deck, now)?;
        let new_used = seen.values().filter(|v| **v).count() as u32;
        let review_used = seen.len() as u32 - new_used;
        let mut new_available = new_limit.saturating_sub(new_used);
        let mut review_available = review_limit.unwrap_or(u32::MAX).saturating_sub(review_used);
        let cards: Vec<_> = self
            .state
            .cards
            .iter()
            .filter(|c| c.deck_id == deck)
            .collect();
        let remaining_new = cards.iter().filter(|c| c.schedule.reps == 0).count();
        let mut due: Vec<_> = cards
            .iter()
            .filter(|c| c.schedule.reps > 0 && c.schedule.due.is_some_and(|d| d <= now))
            .collect();
        due.sort_by_key(|c| (c.schedule.due, c.created_at, &c.id));
        let due_reviews = due.len();
        let mut card_ids = vec![];
        for card in due {
            if seen.contains_key(&card.id) {
                card_ids.push(card.id.clone());
            } else if review_available > 0 {
                card_ids.push(card.id.clone());
                review_available -= 1;
            }
        }
        for card in cards.iter().filter(|c| c.schedule.reps == 0) {
            if new_available == 0 {
                break;
            }
            card_ids.push(card.id.clone());
            new_available -= 1;
        }
        Ok(Queue {
            day,
            card_ids,
            remaining_new,
            due_reviews,
            new_used,
            review_used,
            new_limit,
            review_limit,
            new_bonus,
            review_bonus,
        })
    }
    pub fn grade(&mut self, id: &str, rating: u32, now: i64) -> Result<()> {
        if !(1..=4).contains(&rating) {
            return Err(invalid("自己評価は1〜4で指定してください。"));
        }
        if self.state.reviews.last().is_some_and(|r| r.at > now) {
            return Err(invalid(
                "時計が前回の学習より前になっています。端末の日時を確認してください。",
            ));
        }
        let card = self
            .state
            .cards
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| invalid("カードが見つかりません。"))?;
        if !self
            .queue(&card.deck_id, now)?
            .card_ids
            .iter()
            .any(|c| c == id)
        {
            return Err(invalid("このカードは現在の学習対象ではありません。"));
        }
        let deck = self.deck(&card.deck_id)?;
        let day = self.day(now)?;
        let elapsed_days = if let Some(last) = card.schedule.last_review {
            let from = chrono::NaiveDate::parse_from_str(&self.day(last)?, "%Y-%m-%d")
                .map_err(|_| invalid("日時が不正です。"))?;
            let to = chrono::NaiveDate::parse_from_str(&day, "%Y-%m-%d")
                .map_err(|_| invalid("日時が不正です。"))?;
            to.signed_duration_since(from).num_days().max(0) as u32
        } else {
            0
        };
        let memory =
            card.schedule
                .stability
                .zip(card.schedule.difficulty)
                .map(|(stability, difficulty)| fsrs::MemoryState {
                    stability,
                    difficulty,
                });
        let next =
            fsrs::FSRS::new(&deck.parameters)?.next_states(memory, deck.retention, elapsed_days)?;
        let next = match rating {
            1 => next.again,
            2 => next.hard,
            3 => next.good,
            _ => next.easy,
        };
        let interval = if rating == 1 {
            60
        } else {
            next.interval.round().clamp(1.0, 36500.0) as i64 * 86400
        };
        let after = Schedule {
            stability: Some(next.memory.stability),
            difficulty: Some(next.memory.difficulty),
            last_review: Some(now),
            due: Some(
                now.checked_add(interval)
                    .ok_or_else(|| invalid("日時が範囲外です。"))?,
            ),
            reps: card.schedule.reps + 1,
            lapses: card.schedule.lapses + u32::from(rating == 1 && card.schedule.reps > 0),
        };
        let kind = if self.reviewed_today(&card.deck_id, now)?.contains_key(id) {
            "repeat"
        } else if card.schedule.reps == 0 {
            "new"
        } else {
            "review"
        };
        let review = Review {
            id: Uuid::new_v4().to_string(),
            card_id: id.into(),
            deck_id: card.deck_id.clone(),
            at: now,
            rating,
            elapsed_days,
            study_day: day,
            kind: kind.into(),
            before: card.schedule.clone(),
            after: after.clone(),
            parameters: deck.parameters.clone(),
            algorithm: ALGORITHM.into(),
        };
        self.state
            .cards
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| invalid("カードが見つかりません。"))?
            .schedule = after;
        self.state.reviews.push(review);
        self.state.revision += 1;
        Ok(())
    }
    pub fn undo(&mut self) -> Result<()> {
        let review = self
            .state
            .reviews
            .last()
            .ok_or_else(|| invalid("取り消せる自己評価がありません。"))?;
        let card = self
            .state
            .cards
            .iter_mut()
            .find(|c| c.id == review.card_id)
            .ok_or_else(|| invalid("カードが見つかりません。"))?;
        if card.schedule != review.after {
            return Err(invalid("その後に学習状態が変わったため取り消せません。"));
        }
        card.schedule = review.before.clone();
        self.state.reviews.pop();
        self.state.revision += 1;
        Ok(())
    }
    pub fn configure_deck(
        &mut self,
        id: &str,
        retention: f32,
        new_limit: u32,
        review_limit: Option<u32>,
    ) -> Result<()> {
        if !retention.is_finite() || retention <= 0.0 || retention >= 1.0 {
            return Err(invalid("目標保持率は0%より大きく100%未満にしてください。"));
        }
        for bonus in self.state.bonuses.iter().filter(|b| b.deck_id == id) {
            if new_limit.checked_add(bonus.new).is_none()
                || review_limit.is_some_and(|n| n.checked_add(bonus.review).is_none())
            {
                return Err(invalid("当日上乗せと合わせた枚数が大きすぎます。"));
            }
        }
        let deck = self
            .state
            .decks
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| invalid("単語帳が見つかりません。"))?;
        deck.retention = retention;
        deck.new_limit = new_limit;
        deck.review_limit = review_limit;
        self.state.revision += 1;
        Ok(())
    }
    pub fn add_bonus(&mut self, id: &str, new: u32, review: u32, now: i64) -> Result<()> {
        let deck = self.deck(id)?;
        let day = self.day(now)?;
        let bonus = self
            .state
            .bonuses
            .iter()
            .find(|b| b.deck_id == id && b.day == day);
        let new = bonus
            .map(|b| b.new)
            .unwrap_or(0)
            .checked_add(new)
            .ok_or_else(|| invalid("枚数が大きすぎます。"))?;
        let review = bonus
            .map(|b| b.review)
            .unwrap_or(0)
            .checked_add(review)
            .ok_or_else(|| invalid("枚数が大きすぎます。"))?;
        deck.new_limit
            .checked_add(new)
            .ok_or_else(|| invalid("枚数が大きすぎます。"))?;
        if let Some(limit) = deck.review_limit {
            limit
                .checked_add(review)
                .ok_or_else(|| invalid("枚数が大きすぎます。"))?;
        }
        if let Some(bonus) = self
            .state
            .bonuses
            .iter_mut()
            .find(|b| b.deck_id == id && b.day == day)
        {
            bonus.new = new;
            bonus.review = review;
        } else {
            self.state.bonuses.push(Bonus {
                id: Uuid::new_v4().to_string(),
                deck_id: id.into(),
                day,
                new,
                review,
            });
        }
        self.state.revision += 1;
        Ok(())
    }
    pub fn set_settings(&mut self, settings: Settings) -> Result<()> {
        study_day(0, &settings.timezone, settings.day_start_hour)?;
        self.state.settings = settings;
        self.state.revision += 1;
        Ok(())
    }
    pub fn edit_card(
        &mut self,
        id: &str,
        question: &str,
        answer: &str,
        explanation: &str,
    ) -> Result<()> {
        if question.trim().is_empty() || answer.trim().is_empty() {
            return Err(invalid("問題と答えを入力してください。"));
        }
        let card = self
            .state
            .cards
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| invalid("カードが見つかりません。"))?;
        card.question = question.into();
        card.answer = answer.into();
        card.explanation = explanation.into();
        self.state.revision += 1;
        Ok(())
    }
}
