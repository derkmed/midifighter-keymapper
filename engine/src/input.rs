//! Real `enigo`-backed [`InputSink`] (effectful adapter). The pure logic it
//! relies on — key-token resolution — lives in [`crate::keys`] and is tested
//! there; this module is validated by running against a real machine.

use enigo::{
    Button, Coordinate,
    Direction::{Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};

use midifighter_keymapper_core::action::{InputError, InputSink};
use midifighter_keymapper_core::config::MouseAction;

use crate::keys::{resolve_key, ResolvedKey};

/// Injects keystrokes and mouse actions via `enigo`.
pub struct EnigoSink {
    enigo: Enigo,
}

impl EnigoSink {
    pub fn new() -> Result<Self, InputError> {
        Enigo::new(&Settings::default())
            .map(|enigo| Self { enigo })
            .map_err(|e| InputError(format!("enigo init failed: {e}")))
    }

    fn key(&mut self, token: &str, dir: enigo::Direction) -> Result<(), InputError> {
        let resolved =
            resolve_key(token).ok_or_else(|| InputError(format!("unknown key token: {token:?}")))?;
        self.enigo
            .key(to_enigo_key(resolved), dir)
            .map_err(|e| InputError(e.to_string()))
    }
}

fn to_enigo_key(k: ResolvedKey) -> Key {
    match k {
        ResolvedKey::Char(c) => Key::Unicode(c),
        ResolvedKey::Ctrl => Key::Control,
        ResolvedKey::Shift => Key::Shift,
        ResolvedKey::Alt => Key::Alt,
        ResolvedKey::Meta => Key::Meta,
        ResolvedKey::Enter => Key::Return,
        ResolvedKey::Escape => Key::Escape,
        ResolvedKey::Tab => Key::Tab,
        ResolvedKey::Space => Key::Space,
        ResolvedKey::Backspace => Key::Backspace,
        ResolvedKey::Delete => Key::Delete,
        ResolvedKey::Up => Key::UpArrow,
        ResolvedKey::Down => Key::DownArrow,
        ResolvedKey::Left => Key::LeftArrow,
        ResolvedKey::Right => Key::RightArrow,
        ResolvedKey::Home => Key::Home,
        ResolvedKey::End => Key::End,
        ResolvedKey::PageUp => Key::PageUp,
        ResolvedKey::PageDown => Key::PageDown,
        ResolvedKey::F(n) => match n {
            1 => Key::F1,
            2 => Key::F2,
            3 => Key::F3,
            4 => Key::F4,
            5 => Key::F5,
            6 => Key::F6,
            7 => Key::F7,
            8 => Key::F8,
            9 => Key::F9,
            10 => Key::F10,
            11 => Key::F11,
            _ => Key::F12,
        },
    }
}

impl InputSink for EnigoSink {
    fn chord_tap(&mut self, keys: &[String]) -> Result<(), InputError> {
        // Press all keys in order, then release in reverse — a held combo.
        for k in keys {
            self.key(k, Press)?;
        }
        for k in keys.iter().rev() {
            self.key(k, Release)?;
        }
        Ok(())
    }

    fn chord_down(&mut self, keys: &[String]) -> Result<(), InputError> {
        for k in keys {
            self.key(k, Press)?;
        }
        Ok(())
    }

    fn chord_up(&mut self, keys: &[String]) -> Result<(), InputError> {
        for k in keys.iter().rev() {
            self.key(k, Release)?;
        }
        Ok(())
    }

    fn text(&mut self, text: &str) -> Result<(), InputError> {
        self.enigo
            .text(text)
            .map_err(|e| InputError(e.to_string()))
    }

    fn delay(&mut self, ms: u64) -> Result<(), InputError> {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        Ok(())
    }

    fn mouse(&mut self, action: &MouseAction) -> Result<(), InputError> {
        match action {
            MouseAction::LeftClick => self
                .enigo
                .button(Button::Left, enigo::Direction::Click)
                .map_err(|e| InputError(e.to_string())),
            MouseAction::RightClick => self
                .enigo
                .button(Button::Right, enigo::Direction::Click)
                .map_err(|e| InputError(e.to_string())),
            MouseAction::MiddleClick => self
                .enigo
                .button(Button::Middle, enigo::Direction::Click)
                .map_err(|e| InputError(e.to_string())),
            MouseAction::MoveTo { x, y } => self
                .enigo
                .move_mouse(*x, *y, Coordinate::Abs)
                .map_err(|e| InputError(e.to_string())),
        }
    }
}
