use std::collections::HashSet;

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use worker::*;

mod utils;

#[derive(Serialize, Deserialize, Default)]
pub struct Dictionary {
    pub words: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Daily {
    pub word: String,
    #[serde(with = "ts_milliseconds")]
    pub date: DateTime<Utc>,
    pub count: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Previous {
    pub previous: Vec<Daily>,
}

#[event(scheduled)]
pub async fn main(event: ScheduledEvent, env: Env, _ctx: worker::ScheduleContext) {
    utils::set_panic_hook();
    let wordle_namespace = env.kv("wordle").expect("can not get wordle namespace");
    let dictionary = wordle_namespace
        .get("dictionary")
        .json::<Dictionary>()
        .await
        .expect("can not get dictionary from kv");
    let today = wordle_namespace
        .get("today_tr")
        .json::<Daily>()
        .await
        .expect("can not get daily from kv");
    let previous = wordle_namespace
        .get("previous_tr")
        .json::<Previous>()
        .await
        .expect("can not get previous from kv");
    if let Some(dict) = dictionary {
        let dictionary_set: HashSet<String> = dict.words.iter().cloned().collect();
        let mut prev = previous.unwrap_or(Previous::default());
        let previous_words_set: HashSet<String> = prev
            .previous
            .iter()
            .map(|daily_word| daily_word.word.clone())
            .collect();

        let difference = dictionary_set
            .difference(&previous_words_set)
            .cloned()
            .collect::<Vec<String>>();

        let mut rng = rand::thread_rng();
        let random_index = rng.gen_range(0..difference.len());

        let next_word = difference[random_index].clone();
        let today_cloned = today.clone();
        let next_word_struct = Daily {
            word: next_word,
            date: Utc::now(),
            count: today.map(|t| t.count).unwrap_or(0) + 1,
        };

        wordle_namespace
            .put("today_tr", next_word_struct)
            .expect("kv daily_tr write preperation failed")
            .execute().await
            .expect("kv daily_tr write operation failed");

        if today_cloned.is_some() {
            prev.previous.push(today_cloned.unwrap());
        }

        wordle_namespace
            .put("previous_tr", prev)
            .expect("kv previous_tr write preperation failed")
            .execute().await
            .expect("kv previous_tr write operation failed");
    }
}
