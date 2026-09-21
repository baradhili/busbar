# Missing symbols — IEC 60617 coverage gaps

Snapshot of the renderer's symbol coverage against (a) the device types
actually used by the corpus and (b) the spec's built-in type tables
(spec §8). This feeds `busbar-render` work; the drafting contract itself
is `layout-guidance.md` §4 (IEC 60617:2024 everywhere; non-standard
symbols cause review delays).

Derived from the corpus `.esld` inventory and the two mapping points —
`glyph_for()` in `crates/busbar-layout/src/lib.rs` and the body-draw arms
in `crates/busbar-render/src/lib.rs`. Re-take this inventory when a
symbol lands.

## Covered — dedicated or correct-family symbols

| Type(s) | Rendering |
|---|---|
| `transformer` | two-circle (IEC 60617-6) |
| `fuse` | rectangle with line through (60617-6) |
| `breaker`, `mcb`, `mccb`, `afci` | switch blade with × release (60617-7) |
| `disconnector`, `isolator`, `load_break_switch` | blade with bar across fixed contact |
| `main_switch` | switch-disconnector: blade + bar + × |
| `contactor` | blade with perpendicular tick |
| `ats`, `changeover` | two-blade transfer assembly |
| `spd` | box with arrow discharging to earth (extracted p18-02) |
| `meter` | meter block with "Wh" legend (sheet `energy-meter1`) |
| `motor` | M-in-circle |
| `lighting` | lamp circle with cross |
| `socket` | outlet semicircle |
| `heating` | box with zigzag |
| `inverter`, `rectifier`, `ups` | converter square with DC/AC marks (60617-7, p11-09) |
| `pv_array`, `pv_string` | PV panel |
| `battery` | battery symbol |
| `earth` | earth bars |
| `junction` | filled dot (tap) |
| `control_relay` | relay-coil box (sheet `relay-coil1`) |
| `rcbo`, `elcb`, `rcmcd` | RCBO: blade + × + residual winding block + arc + "IΔ" (sheet `gfci-breaker1`) |
| `rcd`, `rccb` | blade + residual ellipse + dashed link (sheet `rcd1`) |
| `ct` | current transformer (sheet `CT1`) |
| `wind_turbine` | turbine + rotor (sheet `WT1`) — no longer shares the PV glyph |
| `generator` | "G" box (sheet `G1`) |
| `grid` | "~" supply circle (sheet `L1`) — no longer wears the generator's "G" |
| `evse` | box + socket detail + "EV" (sheet `ev-charger1`) — no IEC 60617 extract exists |
| `board`, `bus`, `busbar` | dashed frame / section bar |


## Missing or weak — exercised by the corpus today

| Type | Corpus uses | Current rendering | Needed | Priority |
|---|---|---|---|---|
| `hvac` | 3 | default plain square | compressor/AHU symbol (or reuse `motor` with a note) | P3 |
| `oven`, `cooktop` | 3 + 1 | default plain square | heating-element box (reuse `heating` glyph) | P3 |
| `pool_pump` | 2 | default plain square | pump symbol (reuse `motor`) | P3 |
| `hws` | 1 | default plain square | hot-water cylinder symbol (or reuse `heating`) | P3 |

## Missing — spec built-ins not yet exercised by the corpus

| Type | Current rendering | Needed | Priority |
|---|---|---|---|
| `relay` (protection relay) | **switch-blade-with-× — the circuit-breaker symbol. Semantically wrong.** The sheet's `time-relay1`/`relay-coil1` are control-side, not protection boxes. | relay box with device-function designations (ANSI/IEC numbers) | **P1** — wrong symbol, not just missing |
| `earth_switch` | plain switch blade | disconnector blade terminating in earth bars | P2 |
| `capacitor_bank` | default plain square (Load kind) | shunt capacitor (two plates) | P3 |
| `reactor` | default plain square | coil / arc-reactor symbol | P3 |
| `ngr` | default plain square | resistor to earth | P3 |
| `vt`, `sync_check` | share the integrating-meter (kWh) circle — a VT drawn as a watt-hour meter | distinct VT / sync-check symbols (CT now has its own) | P3 |
| `bms`, `dc_load` | default plain square | decide when storage chains land (M6) | P4 |
| `cable`, `line` | no glyph (edge entities) | none needed as a placed symbol — wire annotations are the tracked item (guidance §4.4) | — |

## Sheet symbols with no ESLD type yet

The reference sheet (`corpus/render/symbols.svg`) also carries symbols
whose ESLD types do not exist in the spec's built-in registry — tracked
here so the vocabulary is on record for future spec work:
fuse-block (`FBL1`), DC disconnector mark (`dc-disconnector1`),
auxiliary/make contact (`make-contact1`), push-button (`push-button1`),
time relay (`time-relay1`), thermocouple (`thermocouple1`),
AC/DC power supply (`power-supply1`), UPS box (`ups1` — `ups` the type
currently shares the inverter glyph), battery PCS (`PCS1`),
DC combiner (`dc-combiner1`), MPPT charge controller (`mppt1`).

## Adjacent gaps (annotations, not symbol bodies)

The rating note under each glyph (`rating_note`) currently carries
`rating_a` / `kw` / `kvar` / `rcd_ma` only. Guidance §4.3 wants the full
designation string feel; still missing from the note: `curve` (rcbo/mcb),
`rcd_type`, `poles`, `cop` (hvac), and `class` (fuse). Unknown and
user-declared types fall to `Glyph::Generic` → default box + tag, which
is the intended honest fallback (not a gap to close).

## Suggested order of work

1. `relay` — replace the breaker blade with a relay box (wrong-symbol fix).
2. `earth_switch` — blade + earth bars.
3. Load-family specializations (`oven`/`cooktop`/`hws` → heating, `pool_pump` → motor, `hvac` → compressor) — small, corpus-visible.
4. The P3 passive/measurement symbols (`vt`, `sync_check`, `capacitor_bank`, `reactor`, `ngr`) as their corpus usage appears.
5. Sheet vocabulary needing spec types first (UPS box, dc-combiner, mppt…) — follow the spec work.
