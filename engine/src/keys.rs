//! Key-token resolution (pure): map config strings like "ctrl", "f5", "space",
//! "a" to a backend-independent [`ResolvedKey`]. Kept independent of `enigo`'s
//! own `Key` type so it is trivially unit-testable; the `input` adapter maps
//! `ResolvedKey` to `enigo::Key`.

/// A resolved, backend-independent key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedKey {
    Char(char),
    Ctrl,
    Shift,
    Alt,
    Meta,
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// Function key F1..=F12.
    F(u8),
}

/// Resolve a single key token (case-insensitive) to a [`ResolvedKey`].
/// Returns `None` for an unknown token.
pub fn resolve_key(token: &str) -> Option<ResolvedKey> {
    let t = token.trim().to_ascii_lowercase();

    // Function keys: f1..=f12
    if let Some(num) = t.strip_prefix('f') {
        if let Ok(n) = num.parse::<u8>() {
            return (1..=12).contains(&n).then_some(ResolvedKey::F(n));
        }
    }

    Some(match t.as_str() {
        "ctrl" | "control" => ResolvedKey::Ctrl,
        "shift" => ResolvedKey::Shift,
        "alt" | "option" => ResolvedKey::Alt,
        "cmd" | "meta" | "win" | "super" => ResolvedKey::Meta,
        "enter" | "return" => ResolvedKey::Enter,
        "esc" | "escape" => ResolvedKey::Escape,
        "tab" => ResolvedKey::Tab,
        "space" => ResolvedKey::Space,
        "backspace" => ResolvedKey::Backspace,
        "delete" | "del" => ResolvedKey::Delete,
        "up" => ResolvedKey::Up,
        "down" => ResolvedKey::Down,
        "left" => ResolvedKey::Left,
        "right" => ResolvedKey::Right,
        "home" => ResolvedKey::Home,
        "end" => ResolvedKey::End,
        "pageup" | "pgup" => ResolvedKey::PageUp,
        "pagedown" | "pgdn" => ResolvedKey::PageDown,
        _ => {
            // A single character resolves to itself.
            let mut chars = t.chars();
            let c = chars.next()?;
            if chars.next().is_none() {
                return Some(ResolvedKey::Char(c));
            }
            return None;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_modifiers_with_aliases() {
        for t in ["ctrl", "Ctrl", "control", "CONTROL"] {
            assert_eq!(resolve_key(t), Some(ResolvedKey::Ctrl));
        }
        assert_eq!(resolve_key("shift"), Some(ResolvedKey::Shift));
        assert_eq!(resolve_key("alt"), Some(ResolvedKey::Alt));
        for t in ["cmd", "meta", "win", "super"] {
            assert_eq!(resolve_key(t), Some(ResolvedKey::Meta));
        }
    }

    #[test]
    fn resolves_single_characters() {
        assert_eq!(resolve_key("a"), Some(ResolvedKey::Char('a')));
        assert_eq!(resolve_key("Z"), Some(ResolvedKey::Char('z')));
        assert_eq!(resolve_key("5"), Some(ResolvedKey::Char('5')));
    }

    #[test]
    fn resolves_named_keys_with_aliases() {
        for t in ["enter", "return"] {
            assert_eq!(resolve_key(t), Some(ResolvedKey::Enter));
        }
        for t in ["esc", "escape"] {
            assert_eq!(resolve_key(t), Some(ResolvedKey::Escape));
        }
        assert_eq!(resolve_key("tab"), Some(ResolvedKey::Tab));
        assert_eq!(resolve_key("space"), Some(ResolvedKey::Space));
        assert_eq!(resolve_key("backspace"), Some(ResolvedKey::Backspace));
        assert_eq!(resolve_key("delete"), Some(ResolvedKey::Delete));
    }

    #[test]
    fn resolves_arrows_and_navigation() {
        assert_eq!(resolve_key("up"), Some(ResolvedKey::Up));
        assert_eq!(resolve_key("down"), Some(ResolvedKey::Down));
        assert_eq!(resolve_key("left"), Some(ResolvedKey::Left));
        assert_eq!(resolve_key("right"), Some(ResolvedKey::Right));
        assert_eq!(resolve_key("home"), Some(ResolvedKey::Home));
        assert_eq!(resolve_key("end"), Some(ResolvedKey::End));
        assert_eq!(resolve_key("pageup"), Some(ResolvedKey::PageUp));
        assert_eq!(resolve_key("pagedown"), Some(ResolvedKey::PageDown));
    }

    #[test]
    fn resolves_function_keys() {
        assert_eq!(resolve_key("f1"), Some(ResolvedKey::F(1)));
        assert_eq!(resolve_key("F12"), Some(ResolvedKey::F(12)));
    }

    #[test]
    fn rejects_unknown_and_empty() {
        assert_eq!(resolve_key(""), None);
        assert_eq!(resolve_key("wat"), None);
        assert_eq!(resolve_key("f0"), None);
        assert_eq!(resolve_key("f13"), None);
        assert_eq!(resolve_key("ab"), None);
    }
}
