use serde::Deserialize;

use crate::compositor::input::Action;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub keybindings: Vec<KeybindingConfig>,
    #[serde(default)]
    pub floating_app_ids: Vec<String>,
    #[serde(default = "default_super_modifier")]
    pub super_modifier: String,
}

fn default_super_modifier() -> String {
    "logo".to_string()
}

impl Default for ConfigFile {
    fn default() -> Self {
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
            ],
            floating_app_ids: Vec::new(),
            super_modifier: default_super_modifier(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct KeybindingConfig {
    pub action: Action,
    pub key: String,
    #[serde(default)]
    pub modifiers: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_keybindings() {
        let config = ConfigFile::default();
        assert!(!config.keybindings.is_empty());
    }
}
