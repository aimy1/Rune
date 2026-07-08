use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use std::collections::HashMap;
use std::path::Path;

pub struct CalculatorPlugin;

impl CalculatorPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for CalculatorPlugin {
    fn id(&self) -> &'static str {
        "calculator"
    }

    fn name(&self) -> &'static str {
        "Calculator"
    }

    fn description(&self) -> &'static str {
        "Evaluate mathematical expressions"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        // Avoid invoking calculator for letters-only queries
        if !trimmed.chars().any(|c| c.is_ascii_digit()) {
            return Vec::new();
        }

        match evalexpr::eval(trimmed) {
            Ok(value) => {
                let mut metadata = HashMap::new();
                let val_str = value.to_string();
                metadata.insert("result".to_string(), val_str.clone());
                metadata.insert("expression".to_string(), trimmed.to_string());

                vec![SearchResult {
                    id: "calculator_result".to_string(),
                    title: val_str,
                    subtitle: Some(format!("Result of: {trimmed}")),
                    score: 900, // High score so it is near the top
                    plugin_id: self.id(),
                    metadata,
                }]
            }
            Err(_) => Vec::new(),
        }
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let expr = item.metadata.get("expression")?;
        let res = item.metadata.get("result")?;

        Some(format!(
            "# Calculator\n\n**Expression**:\n`{expr}`\n\n**Result**:\n`{res}`\n\n*Press Enter to copy the result to clipboard.*",
        ))
    }

    fn execute(&self, item: &SearchResult, _ctx: &mut Context) -> ExecutionResult {
        if let Some(res) = item.metadata.get("result") {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if clipboard.set_text(res.clone()).is_ok() {
                    return ExecutionResult::Message("Copied result to clipboard!".to_string());
                }
            }
            ExecutionResult::Message("Failed to access clipboard".to_string())
        } else {
            ExecutionResult::Message("Result not found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_calculator() {
        let plugin = CalculatorPlugin::new();
        let cache_dir = PathBuf::from("/tmp");

        let res = plugin.search("2 + 2 * 3", &cache_dir);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "8");

        let res2 = plugin.search("10 / (2 + 3)", &cache_dir);
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].title, "2");

        let res_invalid = plugin.search("firefox", &cache_dir);
        assert!(res_invalid.is_empty());
    }
}
