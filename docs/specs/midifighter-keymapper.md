# Midi Fighter Key-Mapper Utility

## Goal

A personal, cross-platform (Windows + macOS) desktop app for the **Midi Fighter
3D** that maps each grid button to a per-button **macro** it fires into whatever
application is focused, and sets each button's **LED color** live on the device.
A home-grown cousin of the DJ TechTools Midi Fighter Utility, inspired by
midikey2key. Ships from the user's own GitHub repo (`midifighter-keymapper`).

Built on **Rust + Tauri v2** (ADR 0001): `midir` for bidirectional MIDI, `enigo`
for global keystroke/mouse injection, a web frontend for the UI. The riskiest
mechanics — reading presses, lighting pads, firing real key combos — were already
proven end-to-end on the actual hardware during charting (map D6/D7).

## Seams

No code exists yet, so all seams are new. They are kept minimal and pushed to the
**pure core**, with the two effectful edges (MIDI device, input injection) behind
thin adapters so the risk-bearing logic is unit-testable without hardware. Three
seams; `build` attaches its TDD here.

- **S1 — `midi` codec (pure).** `parse(&[u8]) -> Option<DeviceEvent>` and
  `encode_led(bank, cell, Color) -> Vec<u8>`, plus the `note ↔ (bank, cell)`
  mapping. No I/O — operates on byte slices. This is the highest-value seam: the
  protocol is pinned in `docs/reference/midifighter-3d-protocol.md`, so tests
  assert against the exact captured bytes. The `midir` connection layer is a dumb
  adapter that feeds bytes to `parse` and sends bytes from `encode_led`.
- **S2 — `config` (pure).** serde types (`Config`, `Profile`, `Binding`,
  `MacroStep`, `TriggerMode`, `Color`) + `load()`/`save()` + `validate(&Config)`.
  Tests: JSON round-trip, and `validate` **rejects a Hold binding with >1 step**
  (ADR 0002). File I/O is a thin wrapper over the pure serde + validate core.
- **S3 — `action` planner + `InputSink` trait.** `plan(&Binding, PressEvent) ->
  Vec<PlannedAction>` (pure), and an `InputSink` trait the executor drives. Real
  impl wraps `enigo`; a fake `InputSink` is used in tests. Tests attach to the
  planner (step order, delays, Tap-vs-Hold behavior) and to the executor against
  the fake. Keeps all "what fires when" logic out of `enigo`.

Everything else is an intentionally dumb adapter, validated by the proven spike
and a manual run, not unit tests: the `midir` connection layer, the real `enigo`
`InputSink`, the Tauri command bridge, and the web UI.

## Decisions

Every item traces to the map (`docs/maps/midifighter-keymapper.md`) or an ADR.

- **Stack: Rust + Tauri v2**, `midir` 0.10 + `enigo` 0.3 (both proven to build &
  run on Windows), web UI. `directories` for config path, `serde`/`serde_json`
  for config, `macos-accessibility-client` (or ApplicationServices FFI) on mac.
  — map D3, `docs/adr/0001-stack.md`.
- **Device protocol** — grid press = **Note On Ch3** (`0x92 note vel`), release =
  Note Off Ch3; a parallel **Ch4 CC** mirrors each grid button and is **ignored**.
  Bank buttons = **Note On Ch4**, notes 0–3. LEDs are set by echoing a **Ch3 Note
  On of the same note**, velocity = color; Note Off clears the override.
  — map D6, `docs/reference/midifighter-3d-protocol.md`.
- **Bank/cell derivation** — banks re-number grid notes by **+16**, so
  `bank = (note - BASE) / 16`, `cell = (note - BASE) % 16`, with **BASE = 36**
  (standard 3D layout; consistent with the captured 48–67 range). No separate
  bank-state tracking is needed to know which button fired. — map D6.
- **Action model** — a per-button action is a **macro**: an ordered list of steps
  (**chord** / **typed text** / **delay** / **mouse**), with optional repeat; a
  plain chord is a one-step macro. Trigger mode is per button: **Tap** (run the
  macro once on Note On) or **Hold** (keys down on Note On, up on Note Off).
  **Hold is only allowed for single-chord bindings**; a multi-step macro forces
  Tap (UI disables Hold). — map D8, `docs/adr/0002-action-model.md`.
- **Config model** — **multiple named profiles**, manual switch (in-app picker +
  optional hotkey). serde **JSON** in the OS config dir via `directories`. Shape:
  `{ profiles: [Profile], active: id }`; each `Profile` = device settings + a map
  of `(bank, cell) -> Binding { trigger, macro: [MacroStep], color }`. — map D10.
- **UX shape** — top bar (profile picker · connection status · bank tabs 1–4);
  center **4×4 grid** mirroring the active bank (select a pad by click *or by
  physically pressing it*); side panel for the selected pad (Tap/Hold toggle,
  **full multi-step macro editor**, color picker). **Colors apply live** to the
  device on pick. — map D11.
- **macOS Accessibility** — `enigo` silently fails without trust; check
  `AXIsProcessTrusted()`, prompt via `AXIsProcessTrustedWithOptions({prompt:true})`
  and/or open the Privacy pane, poll until granted, and treat a revoked trust as a
  typed error, never a panic. — map D12.
- **Packaging** — Tauri v2 bundler: Windows `nsis`/`msi` (MSI builds on Windows);
  macOS `app`/`dmg` (builds on a Mac or CI macos runner). **Ship unsigned**;
  document the SmartScreen / Gatekeeper click-through in the README. Notarization
  out of scope. — map D13.
- **Color table** — the velocity→color palette is **generated empirically by a
  small sweep tool at build time** from the hardware, not from docs. — map D9.

## Approach

Build in this order; each of S1–S3 is TDD'd against the seam before wiring.

1. **Scaffold** — `cargo` + Tauri v2 project, MIT `LICENSE`, `.gitignore`
   (`target/`, `scratch/`, config artifacts). Commit direct to `main`.
2. **S1 `midi` codec** — `DeviceEvent` (`GridPress{bank,cell}`, `GridRelease`,
   `BankButton{index}`), `parse`, `encode_led`, `note↔(bank,cell)` with `BASE=36`.
   TDD from the captured byte sequences.
3. **S2 `config`** — serde types, defaults, `load`/`save` (path via `directories`),
   and `validate` (Hold ⇒ single step). TDD round-trip + validation.
4. **S3 `action`** — `plan(&Binding, PressEvent)`, `InputSink` trait, `enigo`
   adapter, executor with timing/cancellation. TDD planner + executor-vs-fake.
5. **MIDI connection layer** — open `"Midi Fighter 3D"` in/out (name match, proven),
   feed bytes to `parse`, drive `encode_led` for live color; emit `DeviceEvent`s.
6. **Tauri command bridge** — list/switch profiles, read/write a binding, set a
   pad color live, switch bank, and stream `DeviceEvent`s to the UI.
7. **Frontend UI** — top bar, 4×4 grid mirror, side panel (Tap/Hold, macro editor,
   color picker); physical press selects the pad; color pick applies live.
8. **macOS Accessibility gate** — mac-only: trust check + in-app banner + poll.
9. **Color-table sweep tool** — dev-only utility that steps velocities on one pad
   to capture the palette; its output feeds the color picker.
10. **Packaging** — configure bundler targets; README with the unsigned-run steps.

## Out of scope

- **Low-level / scancode injection** for games & full-screen apps — v1 uses
  `enigo`'s high-level path (proven on Windows). (map fog)
- **Multi-device support** (Twister / 64 / Spectra) — hard-coded to the 3D. (fog)
- **Auto-switch profiles by focused application** — manual switch only; needs
  per-OS foreground-window detection. (map D10 / fog)
- **Auto-launch / system-tray / background running.** (fog)
- **Code signing & notarization** — ship unsigned; document the bypass. (map D13)
- **"Hold last chord" / "loop while held"** macro semantics — Hold is
  single-chord only. (ADR 0002, resolved)
- **Toggle/latch trigger mode** — only Tap and Hold exist. (ADR 0002)
- **Non-Windows runtime verification of injection** until a Mac is available —
  macOS keystroke behavior is design-verified, not yet runtime-tested. (map D12)

## Acceptance

- **S1:** `parse` maps the exact captured bytes to the right `DeviceEvent`
  (`92 34 7F` → `GridPress{bank,cell}` with `BASE=36`; `93 02 7F` → `BankButton{2}`;
  Ch4 CC bytes → ignored); `encode_led` emits `0x92 note vel`; `note↔(bank,cell)`
  round-trips for all 4 banks. A hardware check confirms bank 1 cell 0 = note 36.
- **S2:** a `Config` with profiles + bindings survives a `save`→`load` round-trip;
  `validate` **rejects** a `Hold` binding whose macro has more than one step.
- **S3:** `plan` for a **Tap** 3-step macro yields the 3 steps in order with their
  delays; a **Hold** single-chord yields keys-down on press and keys-up on release;
  the executor drives a fake `InputSink` exactly as planned.
- **Live behavior (manual, on Windows):** launching connects to "Midi Fighter 3D";
  pressing a physical pad selects it in the UI; picking a color lights that pad
  immediately; a mapped Tap button fires its macro into a focused app; a Hold
  single-chord button holds its keys while pressed.
- **Profiles:** creating, naming, switching, and persisting profiles works across
  an app restart.
- **Packaging:** `tauri build` on Windows produces an installer that runs the app;
  the README documents the SmartScreen/Gatekeeper click-through.
- **macOS (design-verified):** with Accessibility ungranted the app shows the
  permission banner and no silent failure; after granting, injection works.
