//! The engine loop: paint configured LED colors, then map incoming presses to
//! macros executed via `enigo`. Banks are implicit — the device re-numbers grid
//! notes per bank, so `midi::parse` already yields the correct `(bank, cell)`.

use std::collections::HashMap;
use std::sync::mpsc;

use midifighter_keymapper_core::action::{execute, plan, PressEvent};
use midifighter_keymapper_core::config::{Binding, Profile};
use midifighter_keymapper_core::midi::{self, DeviceEvent};

use crate::device;
use crate::input::EnigoSink;

/// Run the engine for `profile` until the process is killed (blocks forever).
pub fn run(profile: &Profile) -> Result<(), String> {
    let base = profile.base_note;

    let bindings: HashMap<(u8, u8), &Binding> = profile
        .bindings
        .iter()
        .map(|pb| ((pb.bank, pb.cell), &pb.binding))
        .collect();

    // Paint the configured colors so the layout is visible on the device.
    let mut out = device::open_output()?;
    for pb in &profile.bindings {
        let bytes = midi::encode_led(base, pb.bank, pb.cell, pb.binding.color);
        let _ = out.send(&bytes);
    }

    let (tx, rx) = mpsc::channel();
    let _conn = device::connect_input(base, tx)?; // keep alive for the loop's lifetime

    let mut sink = EnigoSink::new().map_err(|e| e.0)?;

    // Opt-in verbose event logging for debugging (`MFKM_DEBUG=1`).
    let debug = std::env::var_os("MFKM_DEBUG").is_some();

    println!(
        "Engine running for profile {:?} ({} bindings). Press Ctrl+C to quit.",
        profile.name,
        profile.bindings.len()
    );

    for event in rx {
        let (press, bank, cell) = match event {
            DeviceEvent::GridPress { bank, cell } => (PressEvent::Down, bank, cell),
            DeviceEvent::GridRelease { bank, cell } => (PressEvent::Up, bank, cell),
            DeviceEvent::BankButton { index } => {
                if debug {
                    println!("bank button {index}");
                }
                continue;
            }
        };
        if debug {
            let bound = bindings.contains_key(&(bank, cell));
            println!("{event:?} -> bank {bank} cell {cell} (bound: {bound})");
        }
        if let Some(binding) = bindings.get(&(bank, cell)) {
            let actions = plan(binding, press);
            if let Err(e) = execute(&actions, &mut sink) {
                eprintln!("input error on ({bank},{cell}): {}", e.0);
            }
        }
    }

    Ok(())
}
