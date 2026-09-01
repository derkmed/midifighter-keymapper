# Midi Fighter 3D — MIDI protocol (empirically captured)

Captured live from this unit via the F3 spike (`scratch/spike-midi`), Software
Mode = Ableton (per Utility screenshot). Channels are 1-based.

## Grid buttons (the 4×4 = 16 arcade buttons)

On each press the device emits **two** messages simultaneously; on release, the
matching "off" pair:

| Event   | Message (hex) | Meaning                                  |
|---------|---------------|------------------------------------------|
| press   | `92 <note> 7F`| **Note On, Ch3**, velocity 127           |
| press   | `B3 <note> 7F`| CC, Ch4, same number, value 127          |
| release | `82 <note> 7F`| Note Off, Ch3                            |
| release | `B3 <note> 00`| CC, Ch4, value 0                         |

**Use the Ch3 Note On as the trigger source.** The Ch4 CC is a parallel echo of
the same button and can be ignored for key-mapping.

## Banks (the killer detail)

Switching banks **shifts the grid note number by +16 per bank; the channel stays
Ch3.** Observed contiguous grid notes 48–67 (0x30–0x43) across banks — more than
16, which is only possible if banks re-number. Working model (to confirm across
all 4 banks): `note = BASE + 16*(bank_index) + cell`, where `cell` is 0–15 within
the 4×4 grid. Bank 2's range 52–67 was fully observed; Bank 1 tail 48–51 seen.

**Implication for config:** a mapping is keyed by **(bank, cell)** internally, but
on the wire it's just a note number on Ch3. The app can derive bank+cell from the
note (`bank = note/16`, `cell = note%16`) given the base — no separate bank-state
tracking needed to know which button fired.

## Physical cell layout (confirmed via engine smoke test)

`cell = note - base` (base 36), and the physical arrangement of cells on the 4×4
grid is **bottom row first**:

```
row (top)     12 13 14 15
              8  9  10 11
              4  5  6  7
row (bottom)  0  1  2  3
```

The GUI's on-screen grid must map cell 0 to the **bottom-left** pad (not top-left)
so it matches the hardware. Base note 36 confirmed: bank 0 cell 0 == note 36.

## Bank / side buttons

The 4 bank select buttons send **Note On on Ch4**, notes 0–3 (`93 00`..`93 03`),
Note Off `83 0x`. (Ch4 = system/banks/side, matching the DJTT convention.)

## LED color write (mechanism, table TBD)

Light a grid button by sending it a **Note On on Ch3 of the same note number**;
**velocity selects color/animation**. Per DJTT docs: vel 7 ≈ bright red, vel
121–127 forces the button's configured active color, mid ranges = animations
(gate/flash) timed to MIDI clock or ~120 BPM. Send Note Off (Ch3) to clear an
override. **The exact velocity→color table still needs a sweep** — see the map's
fog. Visual confirmation that our echo lit the buttons: PENDING user report.
