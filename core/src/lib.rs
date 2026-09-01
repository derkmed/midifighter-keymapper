//! Pure logic core for the Midi Fighter 3D key-mapper.
//!
//! Three seams (see docs/specs/midifighter-keymapper.md):
//! - [`midi`]   — S1: byte codec + note<->(bank,cell) mapping (pure).
//! - [`config`] — S2: serde schema + validation (pure).
//! - [`action`] — S3: action planner + `InputSink` trait (pure core; enigo impl lives in the app).

pub mod action;
pub mod config;
pub mod edit;
pub mod midi;
pub mod palette;
