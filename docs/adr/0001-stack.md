# ADR 0001 — Build on Rust + Tauri v2

## Status
Accepted (2026-08-31)

## Context
Need a cross-platform (Windows + macOS) desktop utility for the Midi Fighter 3D
that does bidirectional MIDI I/O (read button presses, send Note-back color
messages) and injects synthetic global key combinations. Candidate stacks:
Rust/Tauri (`midir` + `enigo`), JS/Electron (`node-midi`/WebMIDI + `nut.js`),
Python/Qt (`python-rtmidi` + `pynput` + PySide6).

The user is most comfortable in **Python**, but explicitly chose Rust/Tauri for
the best end result.

## Decision
Rust + **Tauri v2**. MIDI via **`midir`**, keystroke injection via **`enigo`**,
GUI as a web frontend inside Tauri. Ships as one small native binary per OS.

## Consequences
- **+** Tiny, self-contained binary; no runtime for the user to install; clean
  GitHub distribution; best-in-class crates for both hard requirements.
- **−** Rust learning curve for a Python-native user → most Rust will be
  AI-generated, raising the value of small, throwaway **prototype spikes** to
  prove `midir` (Note/SysEx round-trip to the device) and `enigo` (global key
  combos on Win + mac) *before* committing to the architecture.
- **−** macOS keystroke injection needs Accessibility permission; must be handled.
- **Fallback**: if the Rust curve outweighs the benefit, Python + Qt is the
  documented escape hatch (`python-rtmidi` + `pynput` + PySide6).
