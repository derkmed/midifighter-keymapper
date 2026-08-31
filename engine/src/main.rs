//! Headless runner for the Midi Fighter key-mapper engine.
//!
//! Loads the saved config's active profile if one exists; otherwise runs a small
//! built-in demo profile so the engine can be smoke-tested against hardware.
//! The GUI (a later slice) will drive the same `engine::run` with edited configs.

use midifighter_keymapper_core::config::{
    self, Binding, MacroStep, PadBinding, Profile, TriggerMode,
};
use midifighter_keymapper_core::midi::{Color, DEFAULT_BASE_NOTE};
use midifighter_keymapper_engine::run;

fn main() {
    let profile = load_active_profile().unwrap_or_else(|| {
        println!("No saved config found — using built-in demo profile.");
        demo_profile()
    });

    if let Err(e) = run::run(&profile) {
        eprintln!("engine error: {e}");
        std::process::exit(1);
    }
}

/// Load the active profile from the on-disk config, if any is valid.
fn load_active_profile() -> Option<Profile> {
    let path = config::default_config_path()?;
    let cfg = config::load(&path).ok()?;
    config::validate(&cfg).ok()?;
    let active = cfg.active.as_deref();
    cfg.profiles
        .iter()
        .find(|p| Some(p.id.as_str()) == active)
        .or_else(|| cfg.profiles.first())
        .cloned()
}

/// A minimal demo covering a Tap chord, typed text, and a Hold single-chord.
fn demo_profile() -> Profile {
    let pad = |bank, cell, trigger, steps, color| PadBinding {
        bank,
        cell,
        binding: Binding { trigger, steps, color: Color(color) },
    };
    Profile {
        id: "demo".into(),
        name: "Demo".into(),
        base_note: DEFAULT_BASE_NOTE,
        bindings: vec![
            // Bank 0, cell 0: Tap Ctrl+C, red.
            pad(
                0,
                0,
                TriggerMode::Tap,
                vec![MacroStep::Chord { keys: vec!["ctrl".into(), "c".into()] }],
                7,
            ),
            // Bank 0, cell 1: Tap types "hello ", another color.
            pad(
                0,
                1,
                TriggerMode::Tap,
                vec![MacroStep::Text { text: "hello ".into() }],
                45,
            ),
            // Bank 0, cell 2: Hold Shift (push-to-hold), another color.
            pad(
                0,
                2,
                TriggerMode::Hold,
                vec![MacroStep::Chord { keys: vec!["shift".into()] }],
                74,
            ),
        ],
    }
}
