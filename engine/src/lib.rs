//! Runtime engine for the Midi Fighter key-mapper: wires the pure `core` to the
//! device (`midir`) and to synthetic input (`enigo`).
//!
//! The one pure, unit-tested seam here is [`keys`] (key-token resolution). The
//! `input`, `device`, and `run` modules are thin adapters over `enigo`/`midir`,
//! validated by running against real hardware (spec: "dumb adapters").

pub mod device;
pub mod input;
pub mod keys;
pub mod run;
