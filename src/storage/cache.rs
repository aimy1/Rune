use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryItem {
    pub count: u32,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Storage {
    pub history: HashMap<String, HistoryItem>, // plugin_id:item_id -> usage info
}

impl Storage {
    pub fn load(cache_dir: &Path) -> Self {
        let path = cache_dir.join("history.json");
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(storage) = serde_json::from_str(&content) {
                    return storage;
                }
            }
        }
        Storage::default()
    }

    pub fn save(&self, cache_dir: &Path) {
        let path = cache_dir.join("history.json");
        let _ = fs::create_dir_all(cache_dir);
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }

    pub fn record_use(&mut self, item_key: &str) {
        let entry = self.history.entry(item_key.to_string()).or_insert_with(|| HistoryItem {
            count: 0,
            last_used: Utc::now(),
        });
        entry.count += 1;
        entry.last_used = Utc::now();
    }

    pub fn get_frecency_bonus(&self, item_key: &str) -> i64 {
        if let Some(item) = self.history.get(item_key) {
            let now = Utc::now();
            let duration = now.signed_duration_since(item.last_used);
            let recency_bonus = if duration.num_hours() < 1 {
                100
            } else if duration.num_days() < 1 {
                50
            } else if duration.num_days() < 7 {
                20
            } else {
                5
            };
            (item.count as i64 * 15) + recency_bonus
        } else {
            0
        }
    }
}
