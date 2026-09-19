# Design documents

| Document | Status | What it is |
|---|---|---|
| [deepseek.md](deepseek.md) | Seed (RSLD v0.1) | Original residential single-line-diagram language draft. Historical starting point. |
| [esld-spec.md](esld-spec.md) | Draft v0.1 | **ESLD — general-purpose Electrical Single Line Diagram language.** Grammar, IR, type library (LV→HV, AC/DC), bus sections/ties, states & interlocks, protection model, validation catalog, rendering contract. Supersedes the RSLD scope (see its Appendix A). |
| [esld-implementation.md](esld-implementation.md) | Draft v0.1 | **BusBar** toolchain plan (working name). Rust implementation (decided), `busbar` CLI first, same core to WASM/JS (`@busbar/core`), Cucumber/Gherkin acceptance & conformance suite, milestones M0–M8. |

## Reading order

1. `esld-spec.md` §1–§9 (language and assemblies),
2. `esld-spec.md` §18 (worked examples),
3. `esld-implementation.md` (how it gets built).

## Next artefacts (from spec §closing)

- JSON Schema for the IR
- Conformance corpus (~40 documents, residential → utility substation)
- Reference SVG symbol library keyed to IEC 60617 / IEEE 315
