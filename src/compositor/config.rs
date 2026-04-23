use serde::Deserialize;

use crate::compositor::input::Action;

pub const DEFAULT_CONFIG: &str = r#"# Wowland Configuration
# Default keybindings

super_modifier = "logo"

[gaps]
inner = 5
outer = 10

[workspace]
count = 4

[[keybindings]]
action = "quit"
key = "Q"
modifiers = ["super"]

[[keybindings]]
action = "next-layout"
key = "space"
modifiers = ["super"]

[[keybindings]]
action = "prev-layout"
key = "space"
modifiers = ["super", "shift"]

[[keybindings]]
action = "focus-next"
key = "J"
modifiers = ["super"]

[[keybindings]]
action = "focus-prev"
key = "K"
modifiers = ["super"]

[[keybindings]]
action = "toggle-float"
key = "F"
modifiers = ["super"]

[[keybindings]]
action = "toggle-maximize"
key = "M"
modifiers = ["super", "shift"]

[[keybindings]]
action = "toggle-minimize"
key = "M"
modifiers = ["super"]

[[keybindings]]
action = "close-focused"
key = "W"
modifiers = ["super"]

[[keybindings]]
action = "cycle-opacity"
key = "O"
modifiers = ["super"]

[[keybindings]]
action = "workspace-prev"
key = "Left"
modifiers = ["super"]

[[keybindings]]
action = "workspace-next"
key = "Right"
modifiers = ["super"]

[[keybindings]]
action = "move-to-workspace-prev"
key = "Left"
modifiers = ["super", "shift"]

[[keybindings]]
action = "move-to-workspace-next"
key = "Right"
modifiers = ["super", "shift"]

[[keybindings]]
action = { spawn = "wofi --show drun" }
key = "Return"
modifiers = ["super"]

[[keybindings]]
action = { launcher = "" }
key = "P"
modifiers = ["super"]
"#;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub keybindings: Vec<KeybindingConfig>,
    #[serde(default)]
    pub floating_app_ids: Vec<String>,
    #[serde(default = "default_super_modifier")]
    pub super_modifier: String,
    #[serde(default)]
    pub decoration_focused: Option<String>,
    #[serde(default)]
    pub decoration_unfocused: Option<String>,
    #[serde(default)]
    pub gaps: Option<GapsConfig>,
    #[serde(default)]
    pub workspace: Option<WorkspaceConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct GapsConfig {
    #[serde(default)]
    pub inner: Option<i32>,
    #[serde(default)]
    pub outer: Option<i32>,
}

#[derive(Debug, Deserialize, Default)]
pub struct WorkspaceConfig {
    #[serde(default = "default_workspace_count")]
    pub count: usize,
}

fn default_workspace_count() -> usize {
    4
}

fn default_super_modifier() -> String {
    "logo".to_string()
}

pub fn parse_hex_color(hex: &str) -> Option<smithay::backend::renderer::Color32F> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
    Some(smithay::backend::renderer::Color32F::new(r, g, b, 1.0))
}

impl Default for ConfigFile {
    fn default() -> Self {
        load_config_from_str(DEFAULT_CONFIG).unwrap_or_else(|| {
            tracing::warn!("Failed to parse default config, using hardcoded fallback");
            Self {
                keybindings: vec![
                    KeybindingConfig {
                        action: Action::Quit,
                        key: "Q".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::NextLayout,
                        key: "space".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::PrevLayout,
                        key: "space".to_string(),
                        modifiers: vec!["super".to_string(), "shift".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::FocusNext,
                        key: "J".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::FocusPrev,
                        key: "K".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::ToggleFloat,
                        key: "F".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::ToggleMaximize,
                        key: "M".to_string(),
                        modifiers: vec!["super".to_string(), "shift".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::ToggleMinimize,
                        key: "M".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::CloseFocused,
                        key: "W".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::CycleOpacity,
                        key: "O".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::WorkspacePrev,
                        key: "Left".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::WorkspaceNext,
                        key: "Right".to_string(),
                        modifiers: vec!["super".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::MoveToWorkspacePrev,
                        key: "Left".to_string(),
                        modifiers: vec!["super".to_string(), "shift".to_string()],
                    },
                    KeybindingConfig {
                        action: Action::MoveToWorkspaceNext,
                        key: "Right".to_string(),
                        modifiers: vec!["super".to_string(), "shift".to_string()],
                    },
                ],
                floating_app_ids: Vec::new(),
                super_modifier: default_super_modifier(),
                decoration_focused: None,
                decoration_unfocused: None,
                gaps: None,
                workspace: None,
            }
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct KeybindingConfig {
    pub action: Action,
    pub key: String,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

fn load_config_from_str(raw: &str) -> Option<ConfigFile> {
    toml::from_str(raw).ok()
}

pub fn load_config(path: &str) -> ConfigFile {
    let raw = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => {
            tracing::warn!("Config not found at {path}: {err}");
            return ConfigFile::default();
        }
    };

    match toml::from_str::<ConfigFile>(&raw) {
        Ok(config) => config,
        Err(err) => {
            tracing::warn!("Failed to parse config {path}: {err}");
            ConfigFile::default()
        }
    }
}

pub fn super_is_alt(config: &ConfigFile) -> bool {
    config.super_modifier.eq_ignore_ascii_case("alt")
}

pub fn xdg_config_path() -> std::path::PathBuf {
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(config_home)
            .join("wowland")
            .join("keybindings.toml");
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home)
            .join(".config")
            .join("wowland")
            .join("keybindings.toml");
    }
    std::path::PathBuf::from("config/keybindings.toml")
}

pub fn load_config_with_fallback(config_path: Option<&str>) -> ConfigFile {
    if let Some(path) = config_path {
        let config = load_config(path);
        if !config.keybindings.is_empty() || !config.floating_app_ids.is_empty() {
            return config;
        }
    }

    let xdg_path = xdg_config_path();
    if xdg_path.exists() {
        let config = load_config(xdg_path.to_str().unwrap_or(""));
        if !config.keybindings.is_empty() || !config.floating_app_ids.is_empty() {
            return config;
        }
    }

    ConfigFile::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_keybindings() {
        let config = ConfigFile::default();
        assert!(!config.keybindings.is_empty());
    }
}
