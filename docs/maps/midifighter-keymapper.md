# Map: Midi Fighter Key-Mapper Utility

A personal, cross-platform (Windows + macOS) desktop app for the **Midi Fighter 3D**
that (a) maps each button to a key combination it fires into whatever app is
focused, and (b) sets each button's LED color — a home-grown cousin of the DJ
TechTools Midi Fighter Utility, inspired by [midikey2key](https://midikey2key.de/).
Lives in the user's own GitHub repo.

## Destination

Building can start when every decision below is made and no open question remains:

- **Stack chosen** — language, desktop framework, and the two hard dependencies
  (MIDI I/O incl. SysEx/Note, and cross-platform synthetic keystroke injection).
- **Device protocol pinned** — how buttons/banks report presses, and the exact
  velocity→color/animation table for setting LEDs on the 3D.
- **Keystroke model settled** — what a "key combination" can express (chords,
  sequences, hold-while-pressed vs. tap, modifiers) and how it's injected globally.
- **Config model settled** — where mappings + colors are stored, format, and how
  banks (4) are represented.
- **UX shape agreed** — the minimum screens: device panel, per-button assignment,
  color picker/grid, bank switching. Roughly matching the reference utility.
- **Distribution decided** — repo name, license, how a build is produced per OS,
  and macOS accessibility-permission story.

Once the frontier below is empty, hand to `/spec docs/maps/midifighter-keymapper.md`.

## Decisions so far

- **D1 — LED color mechanism (research).** The 3D lights a button by receiving a
  Note On of that button's own note back on its MIDI channel (default **Ch3**);
  the **velocity** selects a color or animation state (e.g. Ch3 C3 vel 7 = bright
  red; vel 121–127 forces the button's configured "active" color; other ranges =
  gate/flash animations timed to MIDI clock or ~120 BPM). So the app is a
  bidirectional MIDI endpoint, not just an input reader. Full velocity→color
  table still to be transcribed — see F-fog. Source: DJ TechTools 3D User Guide.
- **D2 — Stack landscape (research).** Two viable paths, both with real prior art:
  - **Rust + Tauri v2** — `midir` (RtMidi-style cross-platform MIDI I/O, does
    Note/SysEx) + `enigo` (most popular Rust key-simulation crate, Win/mac/Linux)
    + web UI. Lightweight, single small binary, easy to ship on GitHub.
  - **JS + Electron** — `node-midi`/`JZZ`/WebMIDI + `nut.js` (or robotjs) for
    keystrokes. Heavier runtime, but familiar if the user lives in JS.
  - Prior art to crib from: `k5md/M2KB` (MIDI→keyboard, Windows), `michd/mmpd`
    (cross-platform MIDI macro-pad daemon).
- **D3 — Stack chosen: Rust + Tauri v2 (grilling).** `midir` (MIDI) + `enigo`
  (keystrokes) + web UI. User is Python-native but chose the best-fit tool → Rust
  is largely AI-generated, so we prove the scary crates with throwaway spikes
  before architecting. Fallback = Python/Qt. See `docs/adr/0001-stack.md`.
- **D4 — Repo identity (task).** Repo `midifighter-keymapper`, MIT license,
  private to start (flip to public later). Not scaffolded yet — that's `build`.
- **D5 — Toolchain audit (task).** Verified on this Win11 machine: VS 2022
  Community w/ MSVC toolset 14.34, Windows SDK 10.0.22000, WebView2 151, winget
  1.29 all present. Only **rustup/Rust is missing** — every other Tauri prereq is
  satisfied, so F5 shrinks to a single install.
- **D6 — Toolchain works + device protocol captured (prototype).** rustc 1.98
  installed; `midir` 0.10 and `enigo` 0.3 both build & run on Windows. Live
  capture from the 3D pinned the protocol (full detail in
  `docs/reference/midifighter-3d-protocol.md`): grid = **Note On Ch3** (+ a
  parallel Ch4 CC to ignore); **banks shift the note by +16**, so (bank,cell) is
  derivable from the note alone; bank buttons = Note On Ch4 notes 0–3; LEDs are
  written by echoing a Ch3 Note On of the same note, velocity = color. This
  resolves the config-model's core question and the port-name lookup.
- **D7 — Full physical loop proven end-to-end (prototype).** On real hardware:
  presses read correctly; echoing color **visibly lit the pads** (user confirmed);
  `enigo` typed text **and** fired a Ctrl+A modifier combo into a focused app
  (user confirmed). All three crates work on this Win11 box. Spike retired to
  `scratch/` (throwaway, won't ship). F3 closed.
- **D8 — Action model (grilling).** Per-button action = a **macro** (ordered
  steps: chord / typed text / delay / mouse, with optional repeat); a plain chord
  is a one-step macro. Trigger mode per button = **Tap** (run once) or **Hold**
  (keys down while pad held). No toggle. See `docs/adr/0002-action-model.md`. One
  sub-question deferred to spec: Hold semantics for a *multi-step* macro.
- **D9 — Color-table method (research → decided approach).** The DJTT 3D User
  Guide PDF is scanned/image-only (un-parseable) and text-manual mirrors are
  403-blocked, but the mechanism is already proven (D6/D7) and the hardware is the
  authoritative source. **Decision: generate the velocity→color table empirically
  with a small sweep tool at build time**, not from docs. Unblocks the color UI
  design without pinning 127 entries now.
- **D10 — Config model (grilling).** **Multiple named profiles, manual switch**
  (in-app picker + optional hotkey). Storage: serde **JSON** in the OS config dir
  via the `directories` crate. Shape: top-level `{ profiles: [...], active: id }`;
  each profile = device settings + a map of **(bank,cell) → { trigger: Tap|Hold,
  macro: [steps], color }**. Full schema pinned in `/spec`. Auto-switch-by-app is
  deferred (see fog).
- **D11 — UX shape (grilling).** Layout: top bar (profile picker · connection ·
  bank tabs 1–4); center 4×4 grid mirroring the active bank (click a pad *or*
  physically press it to select); side panel for the selected pad (Tap/Hold, key
  binding, color picker). **Full multi-step macro editor in v1** (add/reorder
  steps, delays, typed text, mouse) — the critical-path UI item. **Colors apply
  live** to the device on pick. Makes ADR-0002's "Hold + multi-step macro"
  question a real UI case → resolve in `/spec`.
- **D12 — macOS Accessibility story (research).** `enigo` keystrokes **silently
  fail** on mac without Accessibility trust. Plan: on mac, check
  `AXIsProcessTrusted()` at startup/before injection; if untrusted, show an in-app
  banner that calls `AXIsProcessTrustedWithOptions({prompt:true})` and/or opens
  `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`,
  then poll until granted. Trust can flip back to false if revoked → surface as a
  typed error, never a panic. App must be a real `.app` bundle (Tauri provides) to
  appear in the list. Rust: `macos-accessibility-client` crate or ApplicationServices FFI.
- **D13 — Packaging per OS (research).** Tauri v2 bundler: **Windows** `nsis`
  (.exe) / `msi` (MSI must be built on Windows via WiX); **macOS** `app` + `dmg`
  (must be built on a Mac — a Mac or a GitHub Actions macos runner). **Posture for
  a personal app: ship unsigned**, document the click-through: Windows SmartScreen
  "More info → Run anyway"; macOS Gatekeeper → self/ad-hoc sign or
  `xattr -dr com.apple.quarantine <app>` / right-click Open. Full notarization
  (paid Apple Developer ID) is out of scope unless distributing beyond own machines.

## Fog of war

Sensed, not yet sharp — pulled to the frontier as upstream decisions land:

- **Global vs. focused injection** nuance: firing into games/full-screen apps,
  and whether any keystrokes need low-level/scan-code injection vs. high-level.
  (enigo's high-level path proven on Windows; games may need SendInput scancodes.)
- **Multi-model future**: whether to structure config so a Twister/64/Spectra
  could be added later, or hard-code the 3D. (Sharpens after UX.)
- **Auto-switch profiles by focused app** (future enhancement, deferred from D10):
  needs per-OS foreground-window detection.
- Auto-launch / system-tray / run-in-background behavior.

## Frontier

**Empty — the fog is clear.** Every Destination condition is met (stack, device
protocol, keystroke/action model, config model, UX shape, distribution). The route
to build has no open decisions.

## Handoff

`/spec docs/maps/midifighter-keymapper.md` — write the spec, identifying test
seams. Two contained items to pin *during* spec/build (not open decisions, just
detail): (1) ADR-0002's Hold-semantics for a multi-step macro; (2) generate the
velocity→color table via the build-time hardware sweep (D9).

The remaining Fog items are explicitly **out of v1 scope** (future enhancements):
game/full-screen scancode injection, multi-model support, auto-switch profiles by
focused app, and auto-launch/tray behavior.
