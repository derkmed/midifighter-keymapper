# Midi Fighter Key-Mapper

A personal, cross-platform desktop utility for the **DJ TechTools Midi Fighter 3D**
that turns each arcade button into a keyboard/mouse **macro** and sets each
button's **LED color** — a home-grown cousin of the official Midi Fighter Utility,
inspired by [midikey2key](https://midikey2key.de/).

Press a pad on the device → it fires your assigned key combo (or a whole macro)
into whatever app is focused. Configure everything from a visual editor, and let
it run in the background from the system tray.

> **Status:** built and verified end-to-end on **Windows** and **macOS** — on a
> Mac the device enumerates over CoreMIDI, pads light, and presses fire their
> combos into other apps once Accessibility is granted (see
> [macOS: Accessibility](#macos-accessibility-permission)). It also packages to a
> `.dmg` / `.app`; only Developer-ID signing + notarization is left (see
> [Roadmap](#roadmap)).

## Features

- **4×4 grid editor** mirroring the device (cell 0 = bottom-left, matching the hardware).
- **Per-pad macros** — a sequence of steps: key **chords**, **typed text**,
  **delays**, and **mouse** clicks; reorder and edit inline.
- **Tap or Hold** per pad — Tap runs the macro once; Hold holds a single chord's
  keys while the pad is held (push-to-talk / gaming).
- **Device-accurate colors** — pick from the Midi Fighter's real color palette;
  pads and LEDs light up live as you choose.
- **Named profiles** — keep separate layouts (e.g. "OBS", "Gaming") and switch between them.
- **Run in the background** — Start/Stop mapping, minimize to the **system tray**,
  and optionally **launch at login** / **auto-start mapping**.
- **Four banks** — the device re-numbers pads per bank; the app follows automatically.

## How it works

The Midi Fighter 3D sends a **Note On on MIDI channel 3** for each grid press
(banks shift the note by +16), and its LEDs are set by echoing a Note On of the
same note back, with the **velocity selecting a color**. The app is a bidirectional
MIDI endpoint built on:

- **[Tauri v2](https://tauri.app/)** — the desktop shell (Rust backend + web UI).
- **[`midir`](https://crates.io/crates/midir)** — cross-platform MIDI I/O.
- **[`enigo`](https://crates.io/crates/enigo)** — cross-platform synthetic input.

The captured protocol is documented in
[`docs/reference/midifighter-3d-protocol.md`](docs/reference/midifighter-3d-protocol.md).

## Project layout

A Cargo workspace of three crates:

| Crate | What it is |
|-------|------------|
| `core/` | Pure logic — MIDI codec, config schema + validation, action planner, color palette. Fully unit-tested. |
| `engine/` | Runtime — `midir` device I/O + `enigo` input injection, wiring the core to hardware. Includes a headless runner. |
| `app/` | The Tauri desktop app (editor GUI) in `app/` + `frontend/`. |

The design history lives in `docs/` (map, spec, ADRs, protocol reference).

## Building & running

### Prerequisites

- **[Rust](https://rustup.rs/)** (stable).
- **Windows:** the MSVC C++ build tools + [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (bundled on Windows 11).
- **macOS:** Xcode command-line tools.

### The app (GUI)

```bash
cargo run -p midifighter-keymapper-app
```

### macOS: Accessibility permission

macOS blocks synthetic keystrokes until the app is a trusted **Accessibility**
client, so `enigo` injection (and thus every mapping) silently no-ops — or fails
to start — without it. When mapping isn't yet trusted, the app shows a banner
with a **Grant access…** button that opens System Settings →
Privacy & Security → **Accessibility** and adds the app to the list; flip its
switch on and the banner clears itself.

macOS keys this permission to a **code-signing identity**, which a bare
`cargo run` binary doesn't have, so the app must run as a real, signed **`.app`
bundle** to appear in the list — build one with `cargo tauri build`
(see [Packaging](#packaging-macos--windows)) and grant *that*. If you re-toggle
after a rebuild and trust seems stale, quit and relaunch the `.app` — TCC
re-checks the signature on launch.

### Packaging (macOS / Windows)

The Tauri bundler produces native installers:

```bash
cargo install tauri-cli --version "^2.0" --locked   # one-time
cargo tauri build                                    # release .app + .dmg (macOS)
```

Artifacts land in `target/release/bundle/` — on macOS, `macos/*.app` and
`dmg/*.dmg` (on Windows, `nsis/*.exe` / `msi/*.msi`). The build **ad-hoc signs**
the macOS app (`bundle.macOS.signingIdentity = "-"` in `tauri.conf.json`) so it
has a stable identity for the Accessibility grant, but it is **not** notarized.

Because it's unsigned by a Developer ID, Gatekeeper will block it on first open.
Bypass it for personal use by either **right-clicking the app → Open** once, or:

```bash
xattr -dr com.apple.quarantine /Applications/MidiFighterKeyMapper.app
```

Full Developer-ID signing + notarization (a paid Apple Developer account) is out
of scope for personal use.

**Automated releases.** Pushing a version tag builds the installers on GitHub's
runners and attaches them to a Release (`.github/workflows/release.yml`, via
[`tauri-action`](https://github.com/tauri-apps/tauri-action)) — a **universal**
macOS `.dmg`/`.app` plus Windows `.msi`/`.exe`, no local toolchain needed:

```bash
# bump the version in app/tauri.conf.json (and app/Cargo.toml) first, then:
git tag v0.1.0
git push origin v0.1.0
```

The workflow needs no secrets (macOS is ad-hoc signed); users install from the
repo's **Releases** page.

### Headless runner (no GUI — runs the saved active profile or a built-in demo)

```bash
cargo run -p midifighter-keymapper-engine --bin mfkm-headless
```

Set `MFKM_DEBUG=1` to log every incoming press.

### Tests

```bash
cargo test
```

## Usage

1. Plug in the Midi Fighter 3D and launch the app.
2. Create a **profile**, click a pad, and add macro steps (Chord / Text / Delay / Mouse).
3. Pick a **color** for the pad — it previews live on the device.
4. Click **Save** to persist, then **Start mapping**.
5. Close the window to keep mapping running in the **tray**; quit from the tray menu.

Config is stored per-OS (on Windows:
`%APPDATA%\midifighter-keymapper\config\config.json`).

## Notes & limitations

- The device shows colors from a **fixed palette selected by velocity**, not arbitrary
  RGB. There is no solid white — the top velocity range (121–127) is reserved by the
  device for the per-pad "active color" / animations.
- Hold mode applies only to a **single-chord** binding.

## Roadmap

- Developer-ID signing + notarization so the `.dmg` opens without the Gatekeeper
  bypass. Building the `.dmg`/`.app` (ad-hoc signed) and the macOS
  Accessibility-permission flow are both done (see above).
- Full-screen / game key injection via low-level scancodes.
- Auto-switch profiles by focused application.

## License

[MIT](LICENSE).
