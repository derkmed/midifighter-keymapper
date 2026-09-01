//! The engine loop: paint configured LED colors, then map incoming presses to
//! macros executed via `enigo`. Banks are implicit — the device re-numbers grid
//! notes per bank, so `midi::parse` already yields the correct `(bank, cell)`.
//!
//! `run` blocks (used by the headless binary). `spawn` runs the same loop on a
//! background thread with a stop signal, returning an [`EngineHandle`] the GUI
//! can stop — device/enigo resources live entirely on that thread (they are not
//! `Send`), so only the flag and join handle cross threads.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use midifighter_keymapper_core::action::{execute, plan, PressEvent};
use midifighter_keymapper_core::config::{Binding, Profile};
use midifighter_keymapper_core::midi::{self, DeviceEvent};
use midir::{MidiInputConnection, MidiOutputConnection};

use crate::device;
use crate::input::EnigoSink;

/// Live engine resources, all owned on one thread.
struct Running<'a> {
    _out: MidiOutputConnection,
    _conn: MidiInputConnection<()>,
    rx: Receiver<DeviceEvent>,
    sink: EnigoSink,
    bindings: HashMap<(u8, u8), &'a Binding>,
    debug: bool,
}

/// Open the device, paint colors, and wire the input — everything that can fail.
fn setup(profile: &Profile) -> Result<Running<'_>, String> {
    let base = profile.base_note;
    let bindings: HashMap<(u8, u8), &Binding> = profile
        .bindings
        .iter()
        .map(|pb| ((pb.bank, pb.cell), &pb.binding))
        .collect();

    let mut out = device::open_output()?;
    for pb in &profile.bindings {
        let _ = out.send(&midi::encode_led(base, pb.bank, pb.cell, pb.binding.color));
    }

    let (tx, rx) = mpsc::channel();
    let conn = device::connect_input(base, tx)?;
    let sink = EnigoSink::new().map_err(|e| e.0)?;

    Ok(Running {
        _out: out,
        _conn: conn,
        rx,
        sink,
        bindings,
        debug: std::env::var_os("MFKM_DEBUG").is_some(),
    })
}

/// Run the event loop until `stop` is set (or the device disconnects).
fn pump(r: &mut Running, stop: &AtomicBool) {
    while !stop.load(Ordering::SeqCst) {
        match r.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => dispatch(event, &r.bindings, &mut r.sink, r.debug),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn dispatch(
    event: DeviceEvent,
    bindings: &HashMap<(u8, u8), &Binding>,
    sink: &mut EnigoSink,
    debug: bool,
) {
    let (press, bank, cell) = match event {
        DeviceEvent::GridPress { bank, cell } => (PressEvent::Down, bank, cell),
        DeviceEvent::GridRelease { bank, cell } => (PressEvent::Up, bank, cell),
        DeviceEvent::BankButton { index } => {
            if debug {
                println!("bank button {index}");
            }
            return;
        }
    };
    if debug {
        let bound = bindings.contains_key(&(bank, cell));
        println!("{event:?} -> bank {bank} cell {cell} (bound: {bound})");
    }
    if let Some(binding) = bindings.get(&(bank, cell)) {
        let actions = plan(binding, press);
        if let Err(e) = execute(&actions, sink) {
            eprintln!("input error on ({bank},{cell}): {}", e.0);
        }
    }
}

/// Run the engine for `profile` until the process is killed (blocks forever).
pub fn run(profile: &Profile) -> Result<(), String> {
    let mut running = setup(profile)?;
    println!(
        "Engine running for profile {:?} ({} bindings). Press Ctrl+C to quit.",
        profile.name,
        profile.bindings.len()
    );
    let never = AtomicBool::new(false);
    pump(&mut running, &never);
    Ok(())
}

/// A running engine on a background thread; call [`EngineHandle::stop`] to end it.
pub struct EngineHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl EngineHandle {
    /// Signal the engine to stop and wait for its thread to finish.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Start the engine on a background thread. Returns once the device is open (or
/// with the setup error), so callers learn immediately if the device is missing.
pub fn spawn(profile: Profile) -> Result<EngineHandle, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

    let join = thread::spawn(move || match setup(&profile) {
        Err(e) => {
            let _ = ready_tx.send(Err(e));
        }
        Ok(mut running) => {
            let _ = ready_tx.send(Ok(()));
            pump(&mut running, &stop_thread);
        }
    });

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(EngineHandle { stop, join: Some(join) }),
        Ok(Err(e)) => {
            let _ = join.join();
            Err(e)
        }
        Err(_) => {
            let _ = join.join();
            Err("engine thread exited before starting".into())
        }
    }
}
