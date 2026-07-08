use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub border: String,
    pub selection: String,
    pub warning: String,
    pub success: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct ThemeStyles {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub border: Color,
    pub selection: Color,
    pub warning: Color,
    pub success: Color,
    pub error: Color,
}

fn parse_hex_color(s: &str, default: Color) -> Color {
    let cleaned = s.trim().trim_start_matches('#');
    if cleaned.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&cleaned[0..2], 16),
            u8::from_str_radix(&cleaned[2..4], 16),
            u8::from_str_radix(&cleaned[4..6], 16),
        ) {
            Color::Rgb(r, g, b)
        } else {
            default
        }
    } else {
        default
    }
}

impl ThemeStyles {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            background: parse_hex_color(&theme.background, Color::Reset),
            foreground: parse_hex_color(&theme.foreground, Color::White),
            accent: parse_hex_color(&theme.accent, Color::Blue),
            border: parse_hex_color(&theme.border, Color::DarkGray),
            selection: parse_hex_color(&theme.selection, Color::Indexed(8)),
            warning: parse_hex_color(&theme.warning, Color::Yellow),
            success: parse_hex_color(&theme.success, Color::Green),
            error: parse_hex_color(&theme.error, Color::Red),
        }
    }
}

pub fn get_builtin_theme(name: &str) -> Option<Theme> {
    match name.to_lowercase().as_str() {
        "tokyo_night" | "tokyonight" => Some(Theme {
            background: "#1a1b26".to_string(),
            foreground: "#c0caf5".to_string(),
            accent: "#7aa2f7".to_string(),
            border: "#3b4261".to_string(),
            selection: "#2e3c64".to_string(),
            warning: "#e0af68".to_string(),
            success: "#9ece6a".to_string(),
            error: "#f7768e".to_string(),
        }),
        "catppuccin" => Some(Theme {
            background: "#1e1e2e".to_string(),
            foreground: "#cdd6f4".to_string(),
            accent: "#cba6f7".to_string(),
            border: "#585b70".to_string(),
            selection: "#313244".to_string(),
            warning: "#f9e2af".to_string(),
            success: "#a6e3a1".to_string(),
            error: "#f38ba8".to_string(),
        }),
        "nord" => Some(Theme {
            background: "#2e3440".to_string(),
            foreground: "#d8dee9".to_string(),
            accent: "#88c0d0".to_string(),
            border: "#4c566a".to_string(),
            selection: "#434c5e".to_string(),
            warning: "#ebcb8b".to_string(),
            success: "#a3be8c".to_string(),
            error: "#bf616a".to_string(),
        }),
        "gruvbox" => Some(Theme {
            background: "#282828".to_string(),
            foreground: "#ebdbb2".to_string(),
            accent: "#fe8019".to_string(),
            border: "#504945".to_string(),
            selection: "#3c3836".to_string(),
            warning: "#fabd2f".to_string(),
            success: "#b8bb26".to_string(),
            error: "#fb4934".to_string(),
        }),
        "everforest" => Some(Theme {
            background: "#2d353b".to_string(),
            foreground: "#d3c6aa".to_string(),
            accent: "#a7c080".to_string(),
            border: "#475258".to_string(),
            selection: "#3d484d".to_string(),
            warning: "#dbbc7f".to_string(),
            success: "#a7c080".to_string(),
            error: "#e67e80".to_string(),
        }),
        _ => None,
    }
}

pub fn load_theme(theme_name_or_path: &str) -> Theme {
    // 1. Try to fetch as built-in theme
    if let Some(theme) = get_builtin_theme(theme_name_or_path) {
        return theme;
    }

    // 2. Try loading from file
    let path = Path::new(theme_name_or_path);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(theme) = toml::from_str(&content) {
                return theme;
            }
        }
    }

    // 3. Fallback to Catppuccin
    get_builtin_theme("catppuccin").unwrap()
}
