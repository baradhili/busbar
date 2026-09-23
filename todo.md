# To do list:

## Where we are (2026-09-23)

Green and in the default gate: M0 (scaffold, CI, Cucumber), M1 (parser + `fmt`,
round-trip), M3 (`check` R-1xx/R-2xx), M5 (layout v2 + SVG render, IEC subset).
CLI currently exposes `check`, `fmt`, `render`.

Outstanding: M2 `export` leftover (`interchange.feature` still `@incomplete`),
M4 (R-3xx/R-4xx + code profiles), M6 (solve), M7 (WASM), M8 (PDF/ANSI/LSP),
plus the `spec-examples.feature` chore.

## Next phase — M4: protection/source rules + M2 export closeout

### M4 core — `busbar-check` R-3xx / R-4xx (plan §5.3, spec §15)

- [ ] Code-profile engine: merge active `code` blocks into a `CodeParams` map;
      rules read typed keys with spec-declared defaults
- [ ] Unknown code-profile keys preserved (forward compat) but reported as notes
- [ ] Severity override via profiles (e.g. downgrade R-307 to warning) —
      catalog stays frozen, presentation only
- [ ] Unit normalization for rating comparisons at the IR boundary (spec §4.6);
      dimension mismatches (`rating_a = 230V`) → E-level diagnostics
- [ ] R-3xx rules run post-expansion:
  - [ ] R-301 — protective device rating exceeds cable ampacity (error)
  - [ ] R-302 — connected load exceeds protective device rating (error)
  - [ ] R-303 — sum of circuit ratings exceeds board main rating (warning)
  - [ ] R-304 — socket circuit without RCD, code-dependent (error)
  - [ ] R-305 — SPD absent at board incomer, code-dependent (warning)
  - [ ] R-306 — breaking capacity below prospective fault current (error)
  - [ ] R-307 — no main switch/isolator between board incomer and bus (error)
  - [ ] R-308 — RCD selectivity violated upstream/downstream (warning)
  - [ ] R-309 — AFCI required but absent, code-dependent (error)
  - [ ] R-310 — switchgear continuous rating below circuit load (error)
  - [ ] R-311 — transformer kVA below connected load after diversity (warning)
  - [ ] R-312 — current-function relay without CT (error)
  - [ ] R-313 — relay `trips` target is not a switching device (error)
- [ ] R-4xx rules run post-expansion:
  - [ ] R-401 — backfeed source sum exceeds code % of busbar rating (error)
  - [ ] R-402 — more than one neutral-earth bond per islanded system (error)
  - [ ] R-403 — grid-tie inverter without anti-islanding declared (error)
  - [ ] R-404 — battery discharge path to grid without `export_allowed` (error)
  - [ ] R-405 — `essential` circuit not reachable from island-capable source (error)
  - [ ] R-406 — aggregate export limit exceeded by declared sources (warning)
  - [ ] R-407 — generator neutral bonded while grid neutral also bonded (error)
  - [ ] R-408 — transfer device without declared `priority` (warning)
  - [ ] R-409 — backup board fed from a non-island-capable source (error)
  - [ ] R-410 — paralleling without `closed_transition`/compliance basis (error)
- [ ] Author one minimal corpus fixture per rule ID in `corpus/invalid/`
      (r301…r313, r401…r410) — replaces the placeholder `-fixture` rows
- [ ] Fill real severities/line numbers in the Examples tables of
      `rules-protection.feature` and `rules-sources-islands.feature`
- [ ] Untag `@incomplete` from both features; `make cucumber` green
- [ ] Unit tests for the code-profile merge + quantity-math edge cases
      (`mm²`, `µ`, `2.5mm2` vs `2.5 mm2`) — proptest where practical

### M2 closeout — `busbar export` (plan §5.7, spec §17)

- [ ] `export --format ir` — normative JSON IR per spec §7.4
- [ ] `export --format dot` — digraph of the linked IR
- [ ] `export --format csv` — board/circuit schedules
- [ ] `load-ir` — accept exported JSON back (sourceSpan optional);
      reloaded document checks identically
- [ ] Untag `@incomplete` from `interchange.feature`

### Housekeeping riding along

- [ ] Tag the spec's ESLD fences as ` ```esld ` + build the extractor;
      untag `spec-examples.feature` (plan §12.3)
- [ ] Update stale AGENTS.md status line ("M0 scaffold" → current state)

### Definition of done (repo conventions)

- [ ] `make lint && make fmt-check && make test` green
- [ ] CodeRabbit pass over pending work, findings addressed
- [ ] Feature branches (`feat/…`) + conventional commits, merged via PR

## After this phase (not started)

- M6 — `busbar-solve`: states, interlocks, scenarios; R-5xx/R-6xx;
  `corpus/solve/` fixtures; both spec worked examples reproduce
- M7 — WASM `@busbar/core` + npm + playground; bundle ≤ 1.5 MB gz
- M8 — PDF export, ANSI symbol set, tree-sitter grammar + LSP
