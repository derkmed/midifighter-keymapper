//! `midir`-backed device I/O (effectful adapter). Opens the Midi Fighter 3D by
//! name, feeds incoming bytes to the pure [`midi::parse`], and exposes an output
//! connection for LED color writes. Validated by running against real hardware.

use std::sync::mpsc::Sender;

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

use midifighter_keymapper_core::midi::{self, DeviceEvent};

/// The MIDI port name of the device (confirmed via the F3 spike).
pub const DEVICE: &str = "Midi Fighter 3D";

/// Open the output connection to the device (for LED color writes).
pub fn open_output() -> Result<MidiOutputConnection, String> {
    let out = MidiOutput::new("mfkm-out").map_err(|e| e.to_string())?;
    let port = out
        .ports()
        .into_iter()
        .find(|p| out.port_name(p).as_deref() == Ok(DEVICE))
        .ok_or_else(|| format!("output port {DEVICE:?} not found"))?;
    out.connect(&port, "mfkm-out").map_err(|e| e.to_string())
}

/// Open the input connection; each parsed [`DeviceEvent`] is sent on `tx`.
/// Keep the returned connection alive for as long as you want to receive events.
pub fn connect_input(
    base: u8,
    tx: Sender<DeviceEvent>,
) -> Result<MidiInputConnection<()>, String> {
    let mut input = MidiInput::new("mfkm-in").map_err(|e| e.to_string())?;
    input.ignore(midir::Ignore::None);
    let port = input
        .ports()
        .into_iter()
        .find(|p| input.port_name(p).as_deref() == Ok(DEVICE))
        .ok_or_else(|| format!("input port {DEVICE:?} not found"))?;
    input
        .connect(
            &port,
            "mfkm-in",
            move |_timestamp, message, _| {
                if let Some(event) = midi::parse(base, message) {
                    let _ = tx.send(event);
                }
            },
            (),
        )
        .map_err(|e| e.to_string())
}
