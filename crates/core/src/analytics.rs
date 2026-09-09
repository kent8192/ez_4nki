use crate::*;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurvePoint {
    pub days: u32,
    pub retention: f32,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastDay {
    pub day: u32,
    pub reviews: usize,
    pub new_cards: usize,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analytics {
    pub total_reviews: usize,
    pub success_rate: Option<f32>,
    pub learned_cards: usize,
    pub forecast: Vec<ForecastDay>,
    pub optimization_samples: usize,
    pub can_optimize: bool,
}

impl Library {
    pub fn curve(&self, id: &str, now: i64) -> Result<Vec<CurvePoint>> {
        let card = self
            .state
            .cards
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| invalid("カードが見つかりません。"))?;
        let (Some(stability), Some(difficulty), Some(last)) = (
            card.schedule.stability,
            card.schedule.difficulty,
            card.schedule.last_review,
        ) else {
            return Ok(vec![]);
        };
        let parameters = self
            .state
            .reviews
            .iter()
            .rev()
            .find(|r| r.card_id == id)
            .map(|r| r.parameters.as_slice())
            .unwrap_or(&self.deck(&card.deck_id)?.parameters);
        let elapsed = now.saturating_sub(last).max(0) as f32 / 86400.0;
        Ok((0..=30)
            .map(|days| CurvePoint {
                days,
                retention: fsrs::current_retrievability(
                    fsrs::MemoryState {
                        stability,
                        difficulty,
                    },
                    elapsed + days as f32,
                    parameters[20],
                ),
            })
            .collect())
    }
    pub fn analytics(&self, deck: &str, now: i64) -> Result<Analytics> {
        let config = self.deck(deck)?;
        let cards: Vec<_> = self
            .state
            .cards
            .iter()
            .filter(|c| c.deck_id == deck)
            .collect();
        let reviews: Vec<_> = self
            .state
            .reviews
            .iter()
            .filter(|r| r.deck_id == deck)
            .collect();
        let (items, _) = self.training_items(deck)?;
        let optimization_samples = items
            .iter()
            .filter(|i| i.reviews.last().is_some_and(|r| r.delta_t > 0))
            .count();
        let forecast = if cards.is_empty() {
            (0..30)
                .map(|day| ForecastDay {
                    day,
                    reviews: 0,
                    new_cards: 0,
                })
                .collect()
        } else {
            let parameters = Arc::new(config.parameters.clone());
            let existing = cards
                .iter()
                .enumerate()
                .map(|(index, c)| {
                    if let (Some(stability), Some(difficulty), Some(last), Some(due)) = (
                        c.schedule.stability,
                        c.schedule.difficulty,
                        c.schedule.last_review,
                        c.schedule.due,
                    ) {
                        fsrs::Card {
                            id: index as i64 + 1,
                            stability,
                            difficulty,
                            last_date: (last - now) as f32 / 86400.0,
                            due: ((due - now) as f32 / 86400.0).max(0.0),
                            interval: (due - last) as f32 / 86400.0,
                            lapses: c.schedule.lapses,
                            desired_retention: config.retention,
                            parameters: parameters.clone(),
                        }
                    } else {
                        fsrs::Card {
                            id: index as i64 + 1,
                            parameters: parameters.clone(),
                            desired_retention: config.retention,
                            ..Default::default()
                        }
                    }
                })
                .collect();
            let simulation = fsrs::simulate(
                &fsrs::SimulatorConfig {
                    deck_size: cards.len(),
                    learn_span: 30,
                    learn_limit: config.new_limit as usize,
                    review_limit: config.review_limit.unwrap_or(u32::MAX) as usize,
                    new_cards_ignore_review_limit: true,
                    max_cost_perday: f32::MAX,
                    ..Default::default()
                },
                &config.parameters,
                config.retention,
                Some(42),
                Some(existing),
            )?;
            (0..30)
                .map(|day| ForecastDay {
                    day: day as u32,
                    reviews: simulation.review_cnt_per_day[day],
                    new_cards: simulation.learn_cnt_per_day[day],
                })
                .collect()
        };
        Ok(Analytics {
            total_reviews: reviews.len(),
            success_rate: if reviews.is_empty() {
                None
            } else {
                Some(reviews.iter().filter(|r| r.rating > 1).count() as f32 / reviews.len() as f32)
            },
            learned_cards: cards.iter().filter(|c| c.schedule.reps > 0).count(),
            forecast,
            optimization_samples,
            can_optimize: optimization_samples >= 512,
        })
    }
    pub fn training_items(&self, deck: &str) -> Result<(Vec<fsrs::FSRSItem>, Vec<i64>)> {
        self.deck(deck)?;
        let mut items = vec![];
        let mut ids = vec![];
        for (group, card) in self
            .state
            .cards
            .iter()
            .filter(|c| c.deck_id == deck)
            .enumerate()
        {
            let mut prefix = vec![];
            for review in self.state.reviews.iter().filter(|r| r.card_id == card.id) {
                prefix.push(fsrs::FSRSReview {
                    rating: review.rating,
                    delta_t: review.elapsed_days,
                });
                // FSRS training requires a long-term review in every item, even
                // when the latest review itself is a same-day repetition.
                if prefix.len() >= 2 && prefix.iter().any(|r| r.delta_t > 0) {
                    items.push(fsrs::FSRSItem {
                        reviews: prefix.clone(),
                    });
                    ids.push(group as i64);
                }
            }
        }
        Ok((items, ids))
    }
    pub fn optimize(
        &self,
        deck: &str,
        progress: Arc<Mutex<fsrs::CombinedProgressState>>,
    ) -> Result<Vec<f32>> {
        let (train_set, card_ids) = self.training_items(deck)?;
        if train_set
            .iter()
            .filter(|i| i.reviews.last().is_some_and(|r| r.delta_t > 0))
            .count()
            < 512
        {
            return Err(invalid(
                "個人向け調整には翌日以降の復習履歴が512件以上必要です。現在は既定パラメータを使います。",
            ));
        }
        if progress
            .lock()
            .map_err(|_| invalid("計算状態を読み取れません。"))?
            .want_abort
        {
            return Err(invalid("計算を中断しました。"));
        }
        let result = fsrs::compute_parameters(fsrs::ComputeParametersInput {
            train_set,
            card_ids: Some(card_ids),
            progress: Some(progress.clone()),
            ..Default::default()
        })?;
        if progress
            .lock()
            .map_err(|_| invalid("計算状態を読み取れません。"))?
            .want_abort
        {
            return Err(invalid("計算を中断しました。"));
        }
        Ok(result)
    }
    pub fn apply_parameters(
        &mut self,
        deck: &str,
        parameters: Vec<f32>,
        expected: u64,
    ) -> Result<()> {
        if self.state.revision != expected {
            return Err(invalid(
                "計算中にデータが変わりました。もう一度計算してください。",
            ));
        }
        if parameters.len() != 21
            || parameters.iter().any(|p| !p.is_finite())
            || parameters[20] <= 0.0
        {
            return Err(invalid("FSRSパラメータが不正です。"));
        }
        fsrs::FSRS::new(&parameters)?;
        self.state
            .decks
            .iter_mut()
            .find(|d| d.id == deck)
            .ok_or_else(|| invalid("単語帳が見つかりません。"))?
            .parameters = parameters;
        self.state.revision += 1;
        Ok(())
    }
}
