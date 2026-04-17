use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_window_height")]
    pub window_height: u32,
    #[serde(default = "default_margin_bottom")]
    pub margin_bottom: i32,
    #[serde(default = "default_margin_left")]
    pub margin_left: i32,

    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_line_height")]
    pub line_height: f32,

    #[serde(default = "default_bg_color")]
    pub bg_color: u32,
    #[serde(default = "default_border_search_color")]
    pub border_search_color: u32,
    #[serde(default = "default_border_list_color")]
    pub border_list_color: u32,
    #[serde(default = "default_highlight_bg_color")]
    pub highlight_bg_color: u32,
    #[serde(default = "default_text_color")]
    pub text_color: u32,
    #[serde(default = "default_app_tag_color")]
    pub app_tag_color: u32,
    #[serde(default = "default_project_tag_color")]
    pub project_tag_color: u32,
}

fn default_window_width() -> u32 {
    600
}
fn default_window_height() -> u32 {
    400
}
fn default_margin_bottom() -> i32 {
    20
}
fn default_margin_left() -> i32 {
    20
}

fn default_font_size() -> f32 {
    16.0
}
fn default_line_height() -> f32 {
    20.0
}

// Default Catppuccin Macchiato
fn default_bg_color() -> u32 {
    0xFF24273A
}
fn default_border_search_color() -> u32 {
    0xFF8BD5CA
}
fn default_border_list_color() -> u32 {
    0xFF5B6078
}
fn default_highlight_bg_color() -> u32 {
    0xFF363A4F
}
fn default_text_color() -> u32 {
    0xFFCAD3F5
}
fn default_app_tag_color() -> u32 {
    0xFFA6DA95
}
fn default_project_tag_color() -> u32 {
    0xFFC6A0F6
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_width: default_window_width(),
            window_height: default_window_height(),
            margin_bottom: default_margin_bottom(),
            margin_left: default_margin_left(),
            font_size: default_font_size(),
            line_height: default_line_height(),
            bg_color: default_bg_color(),
            border_search_color: default_border_search_color(),
            border_list_color: default_border_list_color(),
            highlight_bg_color: default_highlight_bg_color(),
            text_color: default_text_color(),
            app_tag_color: default_app_tag_color(),
            project_tag_color: default_project_tag_color(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/"));
        let config_dir = PathBuf::from(home).join(".config").join("aooff");
        let config_path = config_dir.join("config.toml");

        if !config_path.exists() {
            let _ = fs::create_dir_all(&config_dir);
            let default_cfg = Config::default();
            if let Ok(toml_str) = toml::to_string_pretty(&default_cfg) {
                let _ = fs::write(&config_path, toml_str);
            }
            return default_cfg;
        }

        let content = fs::read_to_string(&config_path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }
}
