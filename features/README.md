# Feature coverage map

One file per Feature grouping, traced to the design docs
(`../../Design/esld-spec.md` spec §, `../../Design/esld-implementation.md`
plan §). Status: **green** runs in the default gate; `@incomplete` is
excluded by default and runs (red, honestly) via `make cucumber-incomplete`
until its milestone lands — then the tag comes off.

| Feature file | Covers | Status |
|---|---|---|
| `parsing.feature` | spec §4 lexical, §6 grammar (all statements, quantities, includes) | green |
| `roundtrip.feature` | spec §17.1 round-trip invariant (fmt idempotence, token preservation) | green |
| `rules-structural.feature` | spec §15 R-1xx + clean-check coverage | green |
| `rules-voltage-earthing.feature` | spec §15 R-2xx | green |
| `rules-protection.feature` | spec §15 R-3xx (one row per rule) | @incomplete → M4 |
| `rules-sources-islands.feature` | spec §15 R-4xx (one row per rule) | @incomplete → M4 |
| `rules-scenarios.feature` | spec §15 R-5xx | @incomplete → M6 |
| `rules-states-interlocks.feature` | spec §15 R-6xx | @incomplete → M6 |
| `render.feature` | spec §16.2 determinism, golden hashes | green (M5) |
| `layout.feature` | spec §16.1 + layout-guidance §6 drafting invariants | green (M5) |
| `cli.feature` | plan §5.7 command surface + exit codes | green |
| `interchange.feature` | spec §17 export (IR/DOT/CSV), §15.2 schedules | @incomplete → M2 |
| `solve.feature` | spec §10.7, §13, §14 states/interlocks/scenarios | @incomplete → M6 |
| `spec-examples.feature` | spec §18 worked examples must check clean | @incomplete → M1/M3 tooling |
| `wasm.feature` | plan §5.8 @busbar/core parity + bundle budget | @incomplete → M7 |

Corpus conventions live in `../corpus/README.md`; fixture anchors are
asserted in the Examples tables. Unit-level tests (not Gherkin) cover the
lexer/parser tables, robustness (µ, EOF, ranges, trailing comments), the
load-port and expansion contracts, R-113 device behaviour, and the exact
terminal-landing geometry of routes in each crate's `tests/`.
