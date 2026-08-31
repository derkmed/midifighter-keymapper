# ADR 0002 — Per-button action = macro; trigger = Tap or Hold

## Status
Accepted (2026-08-31). The open sub-question is now resolved (see Consequences).

## Context
Deciding what a Midi Fighter 3D button binding can express (map F4). Options ran
from a single key chord up to a full macro engine, and from tap-only to
tap/hold/toggle trigger behavior. `enigo` is already proven to emit text, chords,
key up/down, and (with its mouse API) pointer actions, so this is a product/scope
decision, not a technical one.

## Decision
- **Action = a macro**: an ordered list of steps. Step kinds: key chord
  (modifiers + key, incl. F-keys/arrows/media), typed text, delay, and mouse
  action; with an optional repeat/loop on the macro. A single chord is just a
  one-step macro — the common case stays simple.
- **Trigger mode is per button, Tap or Hold** (no toggle):
  - **Tap** — run the macro once on Note On.
  - **Hold** — hold the binding's keys down for as long as the pad is held
    (Note On → keys down, Note Off → keys up). Push-to-talk / gaming.

## Consequences
- **+** Covers the reference-utility case (chords) and power uses (macros,
  push-to-talk) without a later schema rewrite — steps are the unit from day one.
- **−** Biggest feature in the app: needs a macro editor UI and a step executor
  with timing/cancellation. Mitigation: the schema ships macro-capable, but the
  UI can land **chord-first** and grow the multi-step editor later.
- **RESOLVED (2026-08-31):** Hold mode is **only available for single-chord
  bindings**. If a binding has more than one step, the trigger mode is forced to
  Tap and the UI disables the Hold option. This keeps Hold semantics unambiguous
  (hold exactly those keys) and matches the push-to-talk use case; no "hold last
  chord" or "loop while held" behavior is built.
- Mouse actions in the macro step set are in scope per "full macro engine"; may be
  deferred in the first UI cut.
