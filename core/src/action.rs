//! S3 — action planner + `InputSink` trait (pure core).
//!
//! `plan` turns a `Binding` + a press/release event into an ordered list of
//! concrete `PlannedAction`s (pure, fully testable). `execute` drives an
//! `InputSink` from that list; the real sink wraps `enigo` in the app layer, and
//! tests use a fake sink. Timing is expressed via `PlannedAction::Delay` /
//! `InputSink::delay` so tests observe order without real sleeps.
//!
//! Trigger semantics (ADR 0002): Tap runs the whole macro once on press; Hold
//! (single-chord only, enforced by `config::validate`) presses the keys on Down
//! and releases them on Up.

use crate::config::{Binding, MacroStep, MouseAction, TriggerMode};

/// A physical button transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressEvent {
    Down,
    Up,
}

/// A concrete, executable action produced by [`plan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedAction {
    /// Press the keys together, then release them.
    ChordTap(Vec<String>),
    /// Press the keys and keep them held.
    ChordDown(Vec<String>),
    /// Release previously-held keys.
    ChordUp(Vec<String>),
    /// Type literal text.
    Text(String),
    /// Wait for `ms` milliseconds.
    Delay(u64),
    /// A mouse action.
    Mouse(MouseAction),
}

/// Turn a binding + a press/release event into the actions to execute.
pub fn plan(binding: &Binding, event: PressEvent) -> Vec<PlannedAction> {
    // Hold mode is single-chord only (ADR 0002, enforced by config::validate):
    // hold the keys down on press, release on release.
    if binding.trigger == TriggerMode::Hold {
        if let [MacroStep::Chord { keys }] = binding.steps.as_slice() {
            return match event {
                PressEvent::Down => vec![PlannedAction::ChordDown(keys.clone())],
                PressEvent::Up => vec![PlannedAction::ChordUp(keys.clone())],
            };
        }
        // An invalid Hold binding shouldn't reach here; fall through to Tap.
    }

    // Tap mode: run the whole macro once on press, nothing on release.
    match event {
        PressEvent::Down => binding.steps.iter().map(step_to_action).collect(),
        PressEvent::Up => Vec::new(),
    }
}

fn step_to_action(step: &MacroStep) -> PlannedAction {
    match step {
        MacroStep::Chord { keys } => PlannedAction::ChordTap(keys.clone()),
        MacroStep::Text { text } => PlannedAction::Text(text.clone()),
        MacroStep::Delay { ms } => PlannedAction::Delay(*ms),
        MacroStep::Mouse { action } => PlannedAction::Mouse(action.clone()),
    }
}

/// An error from an input backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputError(pub String);

/// A sink that performs concrete input actions. Real impl wraps `enigo`.
pub trait InputSink {
    fn chord_tap(&mut self, keys: &[String]) -> Result<(), InputError>;
    fn chord_down(&mut self, keys: &[String]) -> Result<(), InputError>;
    fn chord_up(&mut self, keys: &[String]) -> Result<(), InputError>;
    fn text(&mut self, text: &str) -> Result<(), InputError>;
    fn delay(&mut self, ms: u64) -> Result<(), InputError>;
    fn mouse(&mut self, action: &MouseAction) -> Result<(), InputError>;
}

/// Execute a planned action list against a sink, in order.
pub fn execute(actions: &[PlannedAction], sink: &mut dyn InputSink) -> Result<(), InputError> {
    for action in actions {
        match action {
            PlannedAction::ChordTap(keys) => sink.chord_tap(keys)?,
            PlannedAction::ChordDown(keys) => sink.chord_down(keys)?,
            PlannedAction::ChordUp(keys) => sink.chord_up(keys)?,
            PlannedAction::Text(text) => sink.text(text)?,
            PlannedAction::Delay(ms) => sink.delay(*ms)?,
            PlannedAction::Mouse(action) => sink.mouse(action)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TriggerMode;
    use crate::midi::Color;

    fn tap_macro() -> Binding {
        Binding {
            trigger: TriggerMode::Tap,
            steps: vec![
                MacroStep::Chord { keys: vec!["ctrl".into(), "c".into()] },
                MacroStep::Delay { ms: 50 },
                MacroStep::Text { text: "yo".into() },
            ],
            color: Color(7),
        }
    }

    fn hold_chord() -> Binding {
        Binding {
            trigger: TriggerMode::Hold,
            steps: vec![MacroStep::Chord { keys: vec!["space".into()] }],
            color: Color(45),
        }
    }

    #[test]
    fn tap_runs_full_macro_on_down_and_nothing_on_up() {
        assert_eq!(
            plan(&tap_macro(), PressEvent::Down),
            vec![
                PlannedAction::ChordTap(vec!["ctrl".into(), "c".into()]),
                PlannedAction::Delay(50),
                PlannedAction::Text("yo".into()),
            ]
        );
        assert_eq!(plan(&tap_macro(), PressEvent::Up), vec![]);
    }

    #[test]
    fn hold_single_chord_presses_on_down_releases_on_up() {
        assert_eq!(
            plan(&hold_chord(), PressEvent::Down),
            vec![PlannedAction::ChordDown(vec!["space".into()])]
        );
        assert_eq!(
            plan(&hold_chord(), PressEvent::Up),
            vec![PlannedAction::ChordUp(vec!["space".into()])]
        );
    }

    #[derive(Default)]
    struct FakeSink {
        log: Vec<String>,
    }
    impl InputSink for FakeSink {
        fn chord_tap(&mut self, keys: &[String]) -> Result<(), InputError> {
            self.log.push(format!("tap {keys:?}"));
            Ok(())
        }
        fn chord_down(&mut self, keys: &[String]) -> Result<(), InputError> {
            self.log.push(format!("down {keys:?}"));
            Ok(())
        }
        fn chord_up(&mut self, keys: &[String]) -> Result<(), InputError> {
            self.log.push(format!("up {keys:?}"));
            Ok(())
        }
        fn text(&mut self, text: &str) -> Result<(), InputError> {
            self.log.push(format!("text {text}"));
            Ok(())
        }
        fn delay(&mut self, ms: u64) -> Result<(), InputError> {
            self.log.push(format!("delay {ms}"));
            Ok(())
        }
        fn mouse(&mut self, action: &MouseAction) -> Result<(), InputError> {
            self.log.push(format!("mouse {action:?}"));
            Ok(())
        }
    }

    #[test]
    fn execute_drives_sink_in_planned_order() {
        let actions = plan(&tap_macro(), PressEvent::Down);
        let mut sink = FakeSink::default();
        execute(&actions, &mut sink).unwrap();
        assert_eq!(
            sink.log,
            vec![
                "tap [\"ctrl\", \"c\"]".to_string(),
                "delay 50".to_string(),
                "text yo".to_string(),
            ]
        );
    }
}
