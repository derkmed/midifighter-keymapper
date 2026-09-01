# Handoff: macOS support

This is a task brief for a fresh Claude Code session **running on the Mac** to add
and verify macOS support. Everything the app needs is already cross-platform; this
is about (1) confirming it builds/runs on macOS, (2) the Accessibility-permission
UX, and optionally (3) packaging.

## Current state (as of this handoff)

- The app is **fully built and verified on Windows.** All features work: editor
  GUI, per-pad multi-step macros (chord/text/delay/mouse), Tap/Hold, named
  profiles, device-accurate color palette, live editing, Start/Stop mapping,
  system tray, and launch-at-login.
- Repo: https://github.com/derkmed/midifighter-keymapper (public). Work on `main`.
- Full suite is green: `cargo test` (39 tests). Device is a **Midi Fighter 3D**.

## Architecture (so you don't have to rediscover it)

Cargo workspace, three crates:
- `core/` — pure logic (MIDI codec, config schema+validate, action planner,
  color palette). Unit-tested.
- `engine/` — runtime: `midir` device I/O + `enigo` input injection. The real
  input sink is `engine/src/input.rs` (`EnigoSink`). Engine start/stop lives in
  `engine/src/run.rs` (`spawn` / `EngineHandle`).
- `app/` — Tauri v2 desktop app; backend commands in `app/src/main.rs`, vanilla
  JS UI in `frontend/`.

Read `docs/maps/midifighter-keymapper.md` (decision map), `docs/specs/…`,
`docs/adr/`, and `docs/reference/midifighter-3d-protocol.md` for full context.

## Step 1 — Prereqs & build

```bash
# Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
xcode-select --install        # Xcode command-line tools

git clone https://github.com/derkmed/midifighter-keymapper.git
cd midifighter-keymapper
cargo test                    # should be green on macOS too
cargo run -p midifighter-keymapper-app
```

Expected cross-platform notes / likely fixups:
- `tauri.conf.json` has `bundle.active = false`, so `cargo run` should not need
  macOS icons. If `tauri-build` complains about icons on macOS, add an `.icns` or
  keep bundling off for dev.
- `app/icons/icon.ico` is Windows-only; the Windows Resource is `#[cfg(windows)]`
  in tauri-build, so it should be ignored on macOS.
- `tauri-plugin-autostart` already uses `MacosLauncher::LaunchAgent`.
- `enigo` maps `cmd`/`meta` → Cmd already (see `engine/src/keys.rs`).
- `midir` uses CoreMIDI on macOS; confirm the device still enumerates as
  **"Midi Fighter 3D"** (see `engine/src/device.rs::DEVICE`). If the CoreMIDI
  port name differs, that constant may need adjusting.

## Step 2 — Accessibility permission (the real macOS work: ADR/map D12)

`enigo` keystrokes **silently fail** on macOS unless the app is a trusted
Accessibility client. Plan:

- Add a macOS-only check using `AXIsProcessTrusted()` (crate
  `macos-accessibility-client`, or ApplicationServices FFI).
- Tauri commands: `accessibility_status() -> bool` and a
  `request_accessibility()` that calls `AXIsProcessTrustedWithOptions` with the
  prompt option, and/or opens the pane:
  `open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"`.
- Frontend: when untrusted, show a **banner** ("Grant Accessibility to send
  keystrokes") with a button that calls `request_accessibility()`, then poll
  `accessibility_status()` until granted and clear the banner.
- Trust can be revoked at runtime → surface as a typed error, never panic.
- On Windows these commands should be no-ops returning `true` (guard with
  `#[cfg(target_os = "macos")]`).

Note: keystroke injection requires the **.app bundle** (or the terminal running
`cargo run`) to be granted Accessibility in System Settings → Privacy & Security →
Accessibility. During `cargo run`, you grant your terminal; a bundled `.app`
grants itself.

## Step 3 — Verify on hardware

Plug in the 3D, run the app, create a profile, map a pad, Start mapping, and
confirm the combo fires into another app (e.g. TextEdit). Confirm the
Accessibility banner appears when not granted and clears once granted.

## Step 4 — Packaging (optional, ADR/map D13)

`cargo tauri build` (or the bundler) produces `.app` / `.dmg` on macOS. Ship
**unsigned** for personal use; document the Gatekeeper bypass in the README
(`xattr -dr com.apple.quarantine <app>` or right-click → Open). Notarization
(paid Apple Developer ID) is out of scope unless distributing widely.

## Working style

This project was built with strict TDD at the pure seams in `core/` (and
`engine/keys`); the OS-integration glue (device, enigo, tray, Accessibility) is
validated by running, not unit tests. Keep that split. Commit directly to `main`
with clear messages and push. Update the README's Roadmap/status when macOS works.
```
