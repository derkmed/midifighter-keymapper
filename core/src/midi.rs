//! S1 — MIDI codec for the Midi Fighter 3D (pure, no I/O).
//!
//! Protocol (captured live, see docs/reference/midifighter-3d-protocol.md):
//! - Grid press  = Note On  Ch3 (`0x92 note vel`), vel>0
//! - Grid release = Note Off Ch3 (`0x82 note _`) OR Note On Ch3 vel 0
//! - Each grid button also emits a parallel CC on Ch4 (`0xB3 ...`) — IGNORED.
//! - Bank buttons = Note On Ch4 (`0x93 idx vel`), idx 0..3.
//! - LEDs are set by echoing a Note On Ch3 of the same note; velocity = color.
//! - Banks re-number grid notes by +16: note = base + 16*bank + cell.

use serde::{Deserialize, Serialize};

/// Standard 3D base note for bank 0, cell 0. Confirm against hardware at build.
pub const DEFAULT_BASE_NOTE: u8 = 36;

/// Number of cells in the 4x4 grid.
pub const CELLS_PER_BANK: u8 = 16;
/// Number of banks on the 3D.
pub const BANK_COUNT: u8 = 4;

/// An LED color, represented as the MIDI velocity that selects it on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color(pub u8);

/// A decoded event from the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceEvent {
    GridPress { bank: u8, cell: u8 },
    GridRelease { bank: u8, cell: u8 },
    BankButton { index: u8 },
}

const CH3_NOTE_ON: u8 = 0x92;
const CH3_NOTE_OFF: u8 = 0x82;
const CH4_NOTE_ON: u8 = 0x93;

/// Convert a grid note number to `(bank, cell)` given the base note.
/// Returns `None` if the note is outside the 4-bank grid range.
pub fn note_to_cell(base: u8, note: u8) -> Option<(u8, u8)> {
    let span = CELLS_PER_BANK * BANK_COUNT; // 64
    if note < base || note >= base + span {
        return None;
    }
    let offset = note - base;
    Some((offset / CELLS_PER_BANK, offset % CELLS_PER_BANK))
}

/// Convert `(bank, cell)` back to a grid note number given the base note.
pub fn cell_to_note(base: u8, bank: u8, cell: u8) -> u8 {
    base + bank * CELLS_PER_BANK + cell
}

/// Parse a raw MIDI message into a `DeviceEvent`, or `None` if it is not a
/// grid/bank event we act on (e.g. the parallel Ch4 CC).
pub fn parse(base: u8, msg: &[u8]) -> Option<DeviceEvent> {
    if msg.len() < 3 {
        return None;
    }
    let (status, data1, data2) = (msg[0], msg[1], msg[2]);
    match status {
        // Grid buttons on Ch3.
        CH3_NOTE_ON => {
            let (bank, cell) = note_to_cell(base, data1)?;
            if data2 == 0 {
                Some(DeviceEvent::GridRelease { bank, cell })
            } else {
                Some(DeviceEvent::GridPress { bank, cell })
            }
        }
        CH3_NOTE_OFF => {
            let (bank, cell) = note_to_cell(base, data1)?;
            Some(DeviceEvent::GridRelease { bank, cell })
        }
        // Bank/side buttons on Ch4.
        CH4_NOTE_ON if data2 > 0 => Some(DeviceEvent::BankButton { index: data1 }),
        // Everything else (parallel Ch4 CC, bank Note Off, clock, etc.) is ignored.
        _ => None,
    }
}

/// Encode the MIDI bytes to light a grid button in `color`.
pub fn encode_led(base: u8, bank: u8, cell: u8, color: Color) -> Vec<u8> {
    vec![CH3_NOTE_ON, cell_to_note(base, bank, cell), color.0]
}

/// Encode the MIDI bytes to clear a grid button's color override.
pub fn encode_led_clear(base: u8, bank: u8, cell: u8) -> Vec<u8> {
    vec![CH3_NOTE_OFF, cell_to_note(base, bank, cell), 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_grid_press_on_ch3() {
        // 0x34 = 52 -> (52-36)=16 -> bank 1, cell 0
        assert_eq!(
            parse(DEFAULT_BASE_NOTE, &[0x92, 0x34, 0x7F]),
            Some(DeviceEvent::GridPress { bank: 1, cell: 0 })
        );
        // 0x30 = 48 -> (48-36)=12 -> bank 0, cell 12
        assert_eq!(
            parse(DEFAULT_BASE_NOTE, &[0x92, 0x30, 0x7F]),
            Some(DeviceEvent::GridPress { bank: 0, cell: 12 })
        );
    }

    #[test]
    fn note_on_velocity_zero_is_release() {
        assert_eq!(
            parse(DEFAULT_BASE_NOTE, &[0x92, 0x34, 0x00]),
            Some(DeviceEvent::GridRelease { bank: 1, cell: 0 })
        );
    }

    #[test]
    fn parses_grid_release_on_ch3() {
        assert_eq!(
            parse(DEFAULT_BASE_NOTE, &[0x82, 0x34, 0x7F]),
            Some(DeviceEvent::GridRelease { bank: 1, cell: 0 })
        );
    }

    #[test]
    fn parses_bank_button_on_ch4() {
        assert_eq!(
            parse(DEFAULT_BASE_NOTE, &[0x93, 0x02, 0x7F]),
            Some(DeviceEvent::BankButton { index: 2 })
        );
    }

    #[test]
    fn ignores_parallel_ch4_cc() {
        // The parallel CC mirror of a grid button must not be treated as an event.
        assert_eq!(parse(DEFAULT_BASE_NOTE, &[0xB3, 0x41, 0x7F]), None);
        assert_eq!(parse(DEFAULT_BASE_NOTE, &[0xB3, 0x41, 0x00]), None);
    }

    #[test]
    fn ignores_out_of_range_grid_note() {
        // base+64 = 100 is the first out-of-grid note.
        assert_eq!(parse(DEFAULT_BASE_NOTE, &[0x92, 100, 0x7F]), None);
    }

    #[test]
    fn ignores_too_short_message() {
        assert_eq!(parse(DEFAULT_BASE_NOTE, &[0x92]), None);
        assert_eq!(parse(DEFAULT_BASE_NOTE, &[]), None);
    }

    #[test]
    fn encodes_led_as_ch3_note_on() {
        assert_eq!(
            encode_led(DEFAULT_BASE_NOTE, 1, 0, Color(7)),
            vec![0x92, 52, 7]
        );
    }

    #[test]
    fn encodes_led_clear_as_ch3_note_off() {
        assert_eq!(encode_led_clear(DEFAULT_BASE_NOTE, 1, 0), vec![0x82, 52, 0]);
    }

    #[test]
    fn note_and_cell_round_trip_across_all_banks() {
        for bank in 0..BANK_COUNT {
            for cell in 0..CELLS_PER_BANK {
                let note = cell_to_note(DEFAULT_BASE_NOTE, bank, cell);
                assert_eq!(note_to_cell(DEFAULT_BASE_NOTE, note), Some((bank, cell)));
            }
        }
    }
}
