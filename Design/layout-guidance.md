# ESLD Drawing & Layout Guidance

Normative guidance for the BusBar layout engine (`busbar-layout`) and SVG
renderer (`busbar-render`). Distilled from the reference corpus in
`Design/refs/` and the external references tracked in `todo.md`. This
document drives engine behaviour; the spec (§16) remains the contract.

## Sources

| Ref | What it contributes |
|---|---|
| `refs/24_00726-8…pdf` — Energinet Vidensartikel 1 (DK, 2024) | Minimum SLD content: IEC 60617:2024 symbols; plant/station/field names; voltage level in station names; HV component tags (QA/QB/QC…); drawing number + revision + date; voltage/current/power annotated per component; Fig. 1 = zone-bordered portrait sheet, voltage levels stacked top→bottom |
| `refs/Embedded-Generator-SLD-templates.pdf` — 9 template drawings (Municipal Embedded Generation Support Program) | Bay/column grammar: thick horizontal busbar, vertical feeder columns, incomer devices above the bar, title block bottom-right, border zone ticks, dashed control wiring |
| `refs/isoiecdir2…pdf` — ISO/IEC Directives Part 2 (ed. 9) | Document identification discipline: unique designation, revision control, dated figures; label/key conventions for figures |
| `corpus/render/house.png` — user's hand-drawn reference | Residential-scale arrangement: board frames, busbar with junction-dot taps, cascading load chains, sub-boards below their feeders, dashed control lines |
| Helinks STS design guide (todo item 2) | IEC 61850 hierarchy Substation → Voltage level → Bay → Equipment; bays hang under their busbar; naming `CB_INC_A`, `DS_BT` (function-encoded tags) |
| EEP "Design a LV switchboard" (todo item 2) | Main breaker in its own column; placement follows power reality (supply from below → QF1 at bottom); cable annotations `3 x (3 x 120) Iz = 876 A`; device designation strings |
| Kingsine SCD visualization (todo item 3) | Graphics derive from configuration data, never hand-drawn; layered abstraction; connectivity as traceable paths |
| SurgePV electrical design guide (todo item 4) | IEC 60617 everywhere; series ordering DC (combiner → SPD → fuse → isolator → inverter) and AC (isolator → RCD → MCB → meter → grid); dashed PE; title block bottom-right or drawing is rejected; per-cable cross-sections; multi-line cables = one line |

## 1. Sheet and document conventions

1. **Identification is mandatory.** An SLD is a controlled document: it must
   carry a drawing number, revision, and date (Energinet; ISO/IEC Dir 2).
   *Engine status:* ESLD has no document-metadata statement yet — planned
   `meta { drawing = "…"; revision = "A"; }` (spec change, tracked below).
   Until then the renderer footer states the profile and symbol standard
   only (deterministic — never a generation timestamp).
2. **Title block bottom-right** (all template drawings; SurgePV: operators
   reject SLDs without one).
3. **Border with zone references** on large sheets (letters across,
   numbers down — ISO 5457 style) so equipment can be located by grid ref.
   *Deferred:* only worth drawing once sheets are paginated.
4. **Plant/station/field naming** rides in the title block, not scattered
   in the drawing field.

## 2. Global arrangement grammar

1. **Power flows top → bottom, by voltage level.** Highest voltage at the
   top; each transformation steps down the page (Energinet Fig. 1, EG
   templates 4–6). Boards stack in rank rows below their feeding sources.
2. **Busbars are horizontal, thick bars** (3–4× wire weight); feeders drop
   vertically below them, one column per circuit (every reference agrees).
3. **Bays/columns order left → right: incomers first, then feeders**; on
   multi-section boards the tie sits between the sections it joins. The
   main incomer breaker gets its own dedicated column (EEP).
4. **Incomer devices stack vertically above the busbar**, aligned to the
   incoming wire — main switch closest to the bar, meter/SPD upstream of
   it in power order (EG templates; EEP). Not a horizontal strip: a
   column, matching the vertical flow.
5. **Sub-boards hang below the feeder that feeds them**, and `layout`
   `column` hints fix left→right order (advisory, spec §16.1).
6. **DC-coupled chains** (PV array, battery → inverter) may sit laterally
   beside the inverter (EG template 7); in the rank model they simply
   occupy the row their BFS rank gives them.

## 3. Wire conventions

1. **Orthogonal (Manhattan) routing only** — no diagonals. Elbows bend on
   one axis at a time.
2. **Junction dot at every busbar tap**; a crossing without a dot is *not*
   a connection (universal drafting law).
3. **Solid = power; dashed = control/signal/PE.** Earth conductors dashed
   (SurgePV); peer/control edges dashed (spec §16).
4. **Open arrowheads at load terminations**, oriented along the final wire
   segment, seated at the glyph edge — never inside the symbol.
5. **Line-weight hierarchy:** busbar heaviest, power wires medium, control
   thinnest. *Engine status:* bar is a filled rect, wires 1px, control
   dashed 1px — acceptable at screen scale; formalize if print targets
   arrive.

## 4. Symbols and device designation

1. **IEC 60617:2024 symbols** (Energinet hard requirement; SurgePV:
   non-standard symbols cause review delays). BusBar draws primitives
   (breaker square+diagonal, fuse, two-circle transformer, semicircle
   socket, M-in-circle motor, earth bars).
2. **Tags follow type-prefix discipline** where the author uses it
   (QA/QB/QC/QF breakers-switches, T transformers, M motors, G generators).
   BusBar renders the author's tags verbatim; the `name` property is the
   human-facing label when present.
3. **Ratings ride with the symbol:** rating/curve/sensitivity on a short
   gray note line under the tag (EEP full designation strings are the
   model; today the note line carries `rating_a`/`kw`/`rcd_ma`).
4. **Cable annotations** (size, length, ampacity) belong on the wire, at
   the elbow — *deferred* until ESLD gains cable entities.
5. **Voltage annotation at every level transition**: board label strip
   states its voltage system (`230V 1ph 50Hz`), transformer ports note
   primary/secondary (Energinet: voltage on both sides of transformers).

## 5. Board internals grammar

1. **Frame:** dashed rectangle, name bold top-left inside, voltage-system
   note under the name.
2. **Busbar near the top** of the frame; section bars stacked below each
   other on multi-section boards, tie between them.
3. **One feeder per circuit.** Within a column, devices stack in power
   order top→bottom: protection → controller → loads (matches SurgePV
   AC-side ordering; circuit tag is the column header on the protection
   cell).
4. **Column balancing:** a circuit with more loads than one column should
   hold (≈4 cells) wraps into adjacent sub-columns of the same feeder,
   filling column-major, instead of one unbounded cascade (the "ragged
   cascade" defect in the house reference rendering).
5. **Feed-only circuits** (spare/reserved ways) draw their protection cell
   and stop — a stub, not a full-depth column.
6. **Board-local devices that hang off the bus** (contactor/relay panels,
   board-declared loads) sit in a strip below the feeder band.

## 6. Determinism and machine checks

Layout is a pure function of the IR. Every sort key ends in the unique
tag. The invariant suite (`busbar-layout/tests/invariants.rs`) is the
machine-check for this document:

- glyphs never overlap; members contained in boards; everything on canvas;
- **every resolvable edge is routed** (a dropped route is a dropped wire);
- routes land on the places they name.

Planned additions when the engine work lands: routes are orthogonal;
busbar taps carry junction dots; feeder depth ≤ the balancing threshold.

## 7. Adoption status

**Adopted in the engine now**

| Convention | Where |
|---|---|
| Top→bottom power, horizontal busbars, vertical feeders | rank rows; section bars; feeder columns |
| Incomer column above the bar (vertical, power-ordered) | upstream device column |
| Junction dots at taps; edge-seated arrowheads; dashed control/PE | renderer decorations |
| Dashed board frames; name + voltsys note in label strip | renderer + layout note |
| `name`-property labels; rating note lines | layout `display_label`/`rating_note` |
| Column balancing for long load chains | feeder sub-columns |
| `column` hints; every-edge-routed invariant | layout + tests |

**Tracked for later (spec-level work)**

- `meta` statement (drawing number, revision, date) → renderer title block.
- Cable entities → wire annotations (size/length/Iz).
- Sheet pagination + zone borders for large installations.
- IEC 81346 reference-designation lint (tag prefix ↔ type consistency).
- SCD/IEC 61850 import as an alternate front end (derive graphics from
  configuration data, never hand-drawn — Kingsine lesson).
