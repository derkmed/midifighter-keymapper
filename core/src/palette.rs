//! The device color palette: the Midi Fighter's LEDs show colors from a fixed
//! set selected by MIDI velocity (they can't do arbitrary RGB). This maps a
//! curated set of velocities to a name + an approximate on-screen hex, seeded
//! from the DJTT Spectra/3D color chart and verified against real hardware.

use serde::Serialize;

/// One selectable device color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Swatch {
    pub name: String,
    /// The MIDI velocity that produces this color on the device.
    pub velocity: u8,
    /// Approximate hex for the on-screen swatch/pad (the device shows its own).
    pub hex: String,
}

fn s(name: &str, velocity: u8, hex: &str) -> Swatch {
    Swatch {
        name: name.to_string(),
        velocity,
        hex: hex.to_string(),
    }
}

/// The curated palette, one representative velocity per device color band.
pub fn palette() -> Vec<Swatch> {
    vec![
        s("Off", 6, "#1b1b22"),
        s("Red", 15, "#ff2b2b"),
        s("Dark Red", 21, "#7a1414"),
        s("Orange", 27, "#ff7a1a"),
        s("Dark Orange", 33, "#7a3a0d"),
        s("Yellow", 39, "#ffe02b"),
        s("Dark Yellow", 45, "#8a7a12"),
        s("Green", 57, "#35d84a"),
        s("Dark Green", 69, "#157a25"),
        s("Blue", 81, "#2b8cff"),
        s("Dark Blue", 93, "#16357a"),
        s("Purple", 99, "#a94bff"),
        s("Dark Purple", 105, "#4f1f7a"),
        s("Pink", 111, "#ff4bc0"),
        s("Dark Pink", 117, "#7a1f5c"),
        // Note: the 3D reserves velocities 121-127 for the per-pad "active color"
        // / animations, not a solid white, so there is no white swatch.
    ]
}

/// Display hex for a stored velocity: exact palette match if present, else the
/// nearest palette entry by velocity (so legacy/other values still render).
pub fn hex_for_velocity(v: u8) -> String {
    let p = palette();
    p.iter()
        .find(|sw| sw.velocity == v)
        .or_else(|| p.iter().min_by_key(|sw| (sw.velocity as i16 - v as i16).abs()))
        .map(|sw| sw.hex.clone())
        .unwrap_or_else(|| "#000000".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_is_valid() {
        let p = palette();
        assert!(!p.is_empty());
        // All velocities are within MIDI range.
        assert!(p.iter().all(|s| s.velocity <= 127));
        // Velocities are unique.
        let mut vels: Vec<u8> = p.iter().map(|s| s.velocity).collect();
        vels.sort_unstable();
        vels.dedup();
        assert_eq!(vels.len(), p.len(), "duplicate velocity in palette");
        // Hex values look like #rrggbb.
        assert!(p
            .iter()
            .all(|s| s.hex.len() == 7 && s.hex.starts_with('#')));
    }

    #[test]
    fn hex_for_exact_velocity_matches_swatch() {
        assert_eq!(hex_for_velocity(15), "#ff2b2b"); // Red
        assert_eq!(hex_for_velocity(117), "#7a1f5c"); // Dark Pink
    }

    #[test]
    fn hex_for_unknown_velocity_uses_nearest() {
        // 16 is nearest to Red (15) -> its hex.
        assert_eq!(hex_for_velocity(16), "#ff2b2b");
        // 124 (active-color range) is nearest to Dark Pink (117).
        assert_eq!(hex_for_velocity(124), "#7a1f5c");
    }
}
