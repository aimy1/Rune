use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

pub struct UnitConverterPlugin {
    regex: Regex,
}

impl UnitConverterPlugin {
    pub fn new() -> Self {
        Self {
            regex: Regex::new(
                r"(?i)^\s*([+-]?\d+(?:\.\d+)?)\s*([a-z°]+)\s+(?:to|in)\s+([a-z°]+)\s*$",
            )
            .unwrap(),
        }
    }

    fn convert(&self, value: f64, from: &str, to: &str) -> Option<f64> {
        let from_lower = from.to_lowercase();
        let to_lower = to.to_lowercase();

        // 1. Temperature conversions
        match (from_lower.as_str(), to_lower.as_str()) {
            ("c" | "°c" | "celsius", "f" | "°f" | "fahrenheit") => return Some(value * 1.8 + 32.0),
            ("f" | "°f" | "fahrenheit", "c" | "°c" | "celsius") => return Some((value - 32.0) / 1.8),
            ("c" | "°c" | "celsius", "k" | "kelvin") => return Some(value + 273.15),
            ("k" | "kelvin", "c" | "°c" | "celsius") => return Some(value - 273.15),
            ("f" | "°f" | "fahrenheit", "k" | "kelvin") => return Some((value - 32.0) / 1.8 + 273.15),
            ("k" | "kelvin", "f" | "°f" | "fahrenheit") => return Some((value - 273.15) * 1.8 + 32.0),
            _ => {}
        }

        // 2. Length conversions (base unit: meters)
        let mut lengths = HashMap::new();
        lengths.insert("m", 1.0);
        lengths.insert("meter", 1.0);
        lengths.insert("meters", 1.0);
        lengths.insert("km", 1000.0);
        lengths.insert("kilometer", 1000.0);
        lengths.insert("kilometers", 1000.0);
        lengths.insert("cm", 0.01);
        lengths.insert("centimeter", 0.01);
        lengths.insert("centimeters", 0.01);
        lengths.insert("mm", 0.001);
        lengths.insert("millimeter", 0.001);
        lengths.insert("millimeters", 0.001);
        lengths.insert("mi", 1609.344);
        lengths.insert("mile", 1609.344);
        lengths.insert("miles", 1609.344);
        lengths.insert("yd", 0.9144);
        lengths.insert("yard", 0.9144);
        lengths.insert("yards", 0.9144);
        lengths.insert("ft", 0.3048);
        lengths.insert("foot", 0.3048);
        lengths.insert("feet", 0.3048);
        lengths.insert("in", 0.0254);
        lengths.insert("inch", 0.0254);
        lengths.insert("inches", 0.0254);

        if lengths.contains_key(from_lower.as_str()) && lengths.contains_key(to_lower.as_str()) {
            let from_m = lengths[from_lower.as_str()];
            let to_m = lengths[to_lower.as_str()];
            return Some(value * from_m / to_m);
        }

        // 3. Weight/Mass conversions (base unit: grams)
        let mut masses = HashMap::new();
        masses.insert("g", 1.0);
        masses.insert("gram", 1.0);
        masses.insert("grams", 1.0);
        masses.insert("kg", 1000.0);
        masses.insert("kilogram", 1000.0);
        masses.insert("kilograms", 1000.0);
        masses.insert("mg", 0.001);
        masses.insert("milligram", 0.001);
        masses.insert("lbs", 453.59237);
        masses.insert("lb", 453.59237);
        masses.insert("pound", 453.59237);
        masses.insert("pounds", 453.59237);
        masses.insert("oz", 28.349523);
        masses.insert("ounce", 28.349523);
        masses.insert("ounces", 28.349523);

        if masses.contains_key(from_lower.as_str()) && masses.contains_key(to_lower.as_str()) {
            let from_g = masses[from_lower.as_str()];
            let to_g = masses[to_lower.as_str()];
            return Some(value * from_g / to_g);
        }

        None
    }
}

impl Plugin for UnitConverterPlugin {
    fn id(&self) -> &'static str {
        "unit_converter"
    }

    fn name(&self) -> &'static str {
        "Unit Converter"
    }

    fn description(&self) -> &'static str {
        "Convert between different units"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let caps = match self.regex.captures(query) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let val_str = caps.get(1).unwrap().as_str();
        let from_unit = caps.get(2).unwrap().as_str();
        let to_unit = caps.get(3).unwrap().as_str();

        let value = match val_str.parse::<f64>() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        if let Some(converted) = self.convert(value, from_unit, to_unit) {
            let mut metadata = HashMap::new();
            let result_str = format!("{:.4}", converted)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();

            metadata.insert("result".to_string(), result_str.clone());
            metadata.insert(
                "query".to_string(),
                format!("{value} {from_unit} to {to_unit}"),
            );

            vec![SearchResult {
                id: "unit_conv_result".to_string(),
                title: format!("{result_str} {to_unit}"),
                subtitle: Some(format!("Converted from {value} {from_unit}")),
                score: 950, // Show right under/above calculator
                plugin_id: self.id(),
                metadata,
            }]
        } else {
            Vec::new()
        }
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let query = item.metadata.get("query")?;
        let res = item.metadata.get("result")?;

        Some(format!(
            "# Unit Converter\n\n**Conversion Query**:\n`{query}`\n\n**Result**:\n`{res}`\n\n*Press Enter to copy the conversion result to clipboard.*",
        ))
    }

    fn execute(&self, item: &SearchResult, _ctx: &mut Context) -> ExecutionResult {
        if let Some(res) = item.metadata.get("result") {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if clipboard.set_text(res.clone()).is_ok() {
                    return ExecutionResult::Message("Copied conversion to clipboard!".to_string());
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
    fn test_unit_converter() {
        let plugin = UnitConverterPlugin::new();
        let cache_dir = PathBuf::from("/tmp");

        let res = plugin.search("100 C to F", &cache_dir);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "212 F");

        let res2 = plugin.search("10 km in miles", &cache_dir);
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].title, "6.2137 miles");

        let res_invalid = plugin.search("100 foo to bar", &cache_dir);
        assert!(res_invalid.is_empty());
    }
}
