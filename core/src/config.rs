//! S2 — config schema + validation (pure core; file I/O is a thin wrapper).
//!
//! Shape (see docs/specs/midifighter-keymapper.md, map D10):
//! `Config { active, profiles: [Profile] }`; each `Profile` holds device settings
//! and a list of `(bank, cell) -> Binding { trigger, macro: [MacroStep], color }`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::midi::{Color, DEFAULT_BASE_NOTE};

fn default_base_note() -> u8 {
    DEFAULT_BASE_NOTE
}

/// How a button behaves while pressed (ADR 0002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerMode {
    /// Run the macro once on press.
    #[default]
    Tap,
    /// Hold the (single-chord) binding's keys down while the pad is held.
    Hold,
}

/// A mouse action within a macro. Kept minimal for v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseAction {
    LeftClick,
    RightClick,
    MiddleClick,
    MoveTo { x: i32, y: i32 },
}

/// One step of a macro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MacroStep {
    /// Press a set of keys together (e.g. `["ctrl", "shift", "m"]`).
    Chord { keys: Vec<String> },
    /// Type a literal string.
    Text { text: String },
    /// Wait for `ms` milliseconds.
    Delay { ms: u64 },
    /// Perform a mouse action.
    Mouse { action: MouseAction },
}

/// The action + appearance bound to one pad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    #[serde(default)]
    pub trigger: TriggerMode,
    /// The macro steps. Serialized as `"macro"` (a Rust keyword).
    #[serde(rename = "macro")]
    pub steps: Vec<MacroStep>,
    pub color: Color,
}

/// A binding attached to a specific `(bank, cell)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PadBinding {
    pub bank: u8,
    pub cell: u8,
    pub binding: Binding,
}

/// A named set of pad bindings plus device settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_base_note")]
    pub base_note: u8,
    #[serde(default)]
    pub bindings: Vec<PadBinding>,
}

/// App-wide preferences (not per-profile).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Start the mapping engine automatically when the app launches.
    #[serde(default)]
    pub start_mapping_on_launch: bool,
}

/// Top-level config: all profiles and which one is active.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub settings: Settings,
}

/// A reason a config is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A Hold binding must be exactly one Chord step (ADR 0002).
    HoldRequiresSingleChord { profile: String, bank: u8, cell: u8 },
}

fn is_single_chord(steps: &[MacroStep]) -> bool {
    matches!(steps, [MacroStep::Chord { .. }])
}

/// Validate a config against the invariants that must hold before use.
pub fn validate(cfg: &Config) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    for profile in &cfg.profiles {
        for pad in &profile.bindings {
            if pad.binding.trigger == TriggerMode::Hold && !is_single_chord(&pad.binding.steps) {
                errors.push(ValidationError::HoldRequiresSingleChord {
                    profile: profile.id.clone(),
                    bank: pad.bank,
                    cell: pad.cell,
                });
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Serialize a config to a JSON file, creating parent directories as needed.
pub fn save(cfg: &Config, path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, json)
}

/// Load a config from a JSON file.
pub fn load(path: &Path) -> std::io::Result<Config> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// The default on-disk location for the config (per-OS config dir).
pub fn default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "midifighter-keymapper")
        .map(|d| d.config_dir().join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        Config {
            active: Some("default".into()),
            settings: Settings::default(),
            profiles: vec![Profile {
                id: "default".into(),
                name: "Default".into(),
                base_note: DEFAULT_BASE_NOTE,
                bindings: vec![
                    PadBinding {
                        bank: 0,
                        cell: 0,
                        binding: Binding {
                            trigger: TriggerMode::Tap,
                            steps: vec![
                                MacroStep::Chord { keys: vec!["ctrl".into(), "c".into()] },
                                MacroStep::Delay { ms: 50 },
                                MacroStep::Text { text: "hi".into() },
                            ],
                            color: Color(7),
                        },
                    },
                    PadBinding {
                        bank: 1,
                        cell: 3,
                        binding: Binding {
                            trigger: TriggerMode::Hold,
                            steps: vec![MacroStep::Chord { keys: vec!["v".into()] }],
                            color: Color(45),
                        },
                    },
                ],
            }],
        }
    }

    #[test]
    fn round_trips_through_json_file() {
        let cfg = sample_config();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("config.json");
        save(&cfg, &path).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn serializes_macro_field_and_trigger_names() {
        let json = serde_json::to_string(&sample_config()).unwrap();
        assert!(json.contains("\"macro\""), "macro field renamed: {json}");
        assert!(json.contains("\"tap\""), "tap lowercased: {json}");
        assert!(json.contains("\"hold\""), "hold lowercased: {json}");
    }

    #[test]
    fn validate_rejects_hold_with_multiple_steps() {
        let mut cfg = sample_config();
        // Make the Hold binding multi-step -> invalid.
        cfg.profiles[0].bindings[1].binding.steps = vec![
            MacroStep::Chord { keys: vec!["v".into()] },
            MacroStep::Delay { ms: 10 },
        ];
        let err = validate(&cfg).unwrap_err();
        assert_eq!(
            err,
            vec![ValidationError::HoldRequiresSingleChord {
                profile: "default".into(),
                bank: 1,
                cell: 3,
            }]
        );
    }

    #[test]
    fn validate_rejects_hold_that_is_not_a_chord() {
        let mut cfg = sample_config();
        cfg.profiles[0].bindings[1].binding.steps =
            vec![MacroStep::Text { text: "no".into() }];
        assert!(validate(&cfg).is_err());
    }

    #[test]
    fn settings_default_off_and_backcompat() {
        // A config with no "settings" key (existing on-disk files) still loads,
        // defaulting the setting off.
        let cfg: Config = serde_json::from_str(r#"{"active":null,"profiles":[]}"#).unwrap();
        assert_eq!(cfg.settings.start_mapping_on_launch, false);
        // Round-trips when set.
        let mut cfg2 = Config::default();
        cfg2.settings.start_mapping_on_launch = true;
        let json = serde_json::to_string(&cfg2).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.settings.start_mapping_on_launch, true);
    }

    #[test]
    fn deserializes_frontend_pad_json() {
        // The exact shape the GUI sends to the upsert_binding command.
        let json = r#"{"bank":0,"cell":12,"binding":{"trigger":"tap","macro":[{"type":"chord","keys":["c"]}],"color":7}}"#;
        let pad: PadBinding = serde_json::from_str(json).unwrap();
        assert_eq!(pad.bank, 0);
        assert_eq!(pad.cell, 12);
        assert_eq!(pad.binding.trigger, TriggerMode::Tap);
        assert_eq!(pad.binding.color, Color(7));
    }

    #[test]
    fn validate_accepts_tap_multistep_and_hold_single_chord() {
        // sample_config already has a Tap 3-step and a Hold single-chord.
        assert!(validate(&sample_config()).is_ok());
    }
}
