# ESLD Toolchain — Implementation Plan v0.1

**Status:** Draft for discussion
**Companion to:** [esld-spec.md](esld-spec.md) — ESLD v0.1 language specification
**Scope:** Plan for a reference toolchain implementing the Reader, Validator, Renderer, and Solver conformance classes. **CLI first**, with the same core delivered to **JavaScript/WASM** as a first-class target.

---

## 1. Purpose and constraints

Build a tool that:

1. Parses, validates, formats, renders, and solves `.esld` documents per the v0.1 spec.
2. Ships first as a **native CLI** (`esld`) — single static binary, no runtime deps.
3. Ships the **same core** as a **WASM module** consumable from JavaScript/TypeScript (browser, Node, Deno, edge runtimes), because the natural home of an SLD renderer is the web (viewers, editor integrations, docs sites, review tools).
4. Is deterministic: identical input + options ⇒ identical output (bit-for-bit), per spec §16.2.
5. Is testable against a shared, implementation-neutral **conformance corpus** so that any second implementation (e.g. a pure-TypeScript port) can be certified against the same cases.

### 1.1 Non-goals for v0.1 of the tool

- No GUI editor (the WASM target enables one later; we do not build it).
- No LSP server until M7 (grammar must stabilize first).
- No numeric power flow (spec roadmap 0.5).
- No PDF generation natively in M1–M6; SVG is the normative rendering. PDF arrives via a dedicated pipeline in M8.

---

## 2. Delivery strategy

| Phase | Deliverable | Gate |
|---|---|---|
| **P1** | Native CLI: parse → IR → validate → export (JSON/DOT/CSV), `fmt` | All R-1xx/R-2xx rules green on corpus |
| **P2** | Deterministic SVG renderer (IEC symbol subset), `render` | Golden SVG snapshots stable across platforms |
| **P3** | Solver: states, interlocks, scenarios; R-5xx/R-6xx | Spec worked examples produce expected results |
| **P4** | WASM module + npm package + web playground | Round-trips in browser < 100 ms for 500-node doc |
| **P5** | R-3xx/R-4xx full, code profiles, PDF, LSP | Conformance suite ≥ 40 documents |

The order is deliberate: everything downstream of parsing (validation, rendering, solving) is pure computation over the IR, so the CLI and WASM targets share 100% of the interesting code and differ only in the shell (argv/stdin/stdout vs. JS bindings).

---

## 3. Technology selection

### 3.1 Decision matrix

| Criterion | Rust | Go | TypeScript |
|---|---|---|---|
| Single-file native CLI distribution | ★★★ (static musl binary) | ★★★ | ★☆ (Deno/Bun compile; Node needs runtime) |
| WASM story | ★★★ (`wasm32-unknown-unknown` + `wasm-bindgen`; mature, small, fast) | ★★ (WASM feasible; larger binaries, weaker JS-interop ergonomics) | ★★★ (it *is* JS; no port needed) |
| Serde-style typed JSON IR | ★★★ | ★★ | ★★★ |
| Parser ergonomics | ★★ (hand-rolled or `winnow`/`lalrpop`) | ★★ | ★★ (`tree-sitter` or hand-rolled) |
| Determinism control (iteration order, floats) | ★★★ | ★★ (map iteration randomized) | ★★ (`Map` preserves insertion order; floats IEEE) |
| Contributor pool for an OSS electrical tool | ★★ | ★★★ | ★★★ |
| One codebase, two targets | ★★★ | ★★ | ★★ (CLI distribution is the weak leg) |

### 3.2 Recommendation: Rust core, two thin shells

- `esld-cli` — native binary (`clap`).
- `esld-wasm` — `wasm-bindgen` bindings; published as an npm package with TS types.

The core crates (`syntax`, `ir`, `check`, `solve`, `layout`, `render`) have **no I/O and no wall-clock dependency**: `no_std`-friendly where practical, `std` + `serde` in practice. This is what makes the CLI/WASM split cheap and is a hard architectural rule (§4.3).

### 3.3 Option B — pure TypeScript

If the project pivots JS-first (team composition, embedding in an Electron/VS Code extension from day one), the fallback is a TypeScript monorepo (`@esld/parser`, `@esld/checker`, `@esld/renderer`, `esld-cli` via a Node/Deno/Bun entry). The decision is reversible **only until P4**; after that, both implementations can coexist but must both run the shared conformance corpus (§8) to stay certified. The architecture below is written to be implementable in either language; language-specific notes are marked **[Rust]** / **[TS]**.

---

## 4. System architecture

### 4.1 Repository layout

```
SLD/ (repo "busbar")
├── Design/                     # specs (this document)
├── crates/
│   ├── esld-syntax/            # lexer, parser, AST, trivia, formatter
│   ├── esld-ir/                # model, expansion, JSON (de)serialization
│   ├── esld-check/             # rule engine, diagnostics, code profiles
│   ├── esld-solve/             # states, interlocks, scenarios, reachability
│   ├── esld-layout/            # deterministic layout solver
│   ├── esld-render/            # SVG writer + symbol library data
│   ├── esld-cli/               # native binary
│   └── esld-wasm/              # wasm-bindgen bindings
├── corpus/
│   ├── valid/                  # parse + validate clean
│   ├── invalid/                # expected diagnostics, keyed by rule ID
│   ├── roundtrip/              # fmt(parse(x)) == fmt(parse(fmt(parse(x))))
│   ├── render/                 # golden SVG snapshots
│   └── solve/                  # scenario expectation fixtures
└── tools/                      # corpus runner, snapshot normalizer
```

### 4.2 Pipeline

```
            text (.esld)
               │
        ┌──────▼──────┐
        │ esld-syntax │  lexer → parser → AST (+ trivia, spans)
        └──────┬──────┘
               │ AST
        ┌──────▼──────┐   type defs      ┌─────────────┐
        │  esld-ir    │◄─(built-ins + ───►│ vendor packs│
        │ link/expand │    user types)    └─────────────┘
        └──────┬──────┘
               │ IR (Document)
        ┌──────▼──────┐
        │ esld-check  │  rule engine → diagnostics[]
        └──────┬──────┘
        ┌──────▼──────┐
        │ esld-solve  │  states/interlocks/scenarios → results
        └──────┬──────┘
        ┌──────▼──────┐
        │ esld-layout │  ranked graph → geometry (abstract units)
        └──────┬──────┘
        ┌──────▼──────┐
        │ esld-render │  geometry + symbols → SVG string
        └─────────────┘
```

Each stage is a pure function of its input plus options. No stage writes to disk or clock.

### 4.3 Hard rules

1. **Core purity.** `syntax`…`render` MUST NOT: touch the filesystem (except `syntax` receiving an include-resolver *callback*), read env vars, use time, randomness, threads, or unordered iteration on anything that reaches output.
2. **Determinism.** **[Rust]** Use `BTreeMap`/`IndexMap` (deterministic) — never `HashMap` — for anything serialized or rendered; sort by tag where the spec doesn't order. **[TS]** `Map` iterates in insertion order; never sort with a locale comparator; always emit numbers with fixed formatting (`toFixed`-style, our own formatter).
3. **Diagnostics are values.** Every failure mode (lex, parse, link, rule, solve) produces `Diagnostic { code, severity, message, span, notes }` — never a bare string, never a panic.
4. **No panics across FFI.** WASM entry points catch errors and return structured failures.
5. **Unknown-property preservation.** The AST (not just the IR) is the round-trip substrate; the formatter works from AST + trivia, never from the IR.

### 4.4 Error model

```
Diagnostic
  code      : string      // "E-LEX-3", "R-306", "F-001"…
  severity  : error | warning | note
  message   : string
  span      : Span        // file, byte offsets, line/col (1-based)
  notes     : [string]    // "cable ampacity declared here", etc.
```

Codes: `R-*` are spec rule IDs. `E-*` are tool errors (lex/parse/link/internal). `W-*` tool warnings. The catalog of `E-*` codes lives in `esld-syntax`/`esld-ir` docs and is frozen per release.

---

## 5. Component specifications

### 5.1 `esld-syntax` — lexer, parser, formatter

**Lexer.** Hand-written, span-tracking. Notable decisions:

- **Quantities:** lex `NUMBER` and trailing `UNIT` separately, then *glue* in the parser (`spaced` quantities like `230 V` allowed by spec §4.6). Unit tokenization must not eat identifiers: `400A` → `NUMBER(400) UNIT(A)`; `fault_mva` → `IDENT`.
- **Reserved words** per spec §4.8, contextual after `.` for member names (`GRID.out`).
- Comments and whitespace attach as **trivia** to the following token (leading) / preceding (trailing), preserving round-trip.

**Parser.** Recursive descent following the EBNF one-to-one (the grammar is LL-ish; the only soft spot is `voltsys` positional vs block bodies and `value` vs `ref` in property position — both resolved with one-token lookahead).

- Error recovery: synchronize on `;` and `}` at statement level so one typo yields one diagnostic, not forty.
- AST nodes carry `Span`s and trivia ranges; a `CstView` maps AST ↔ source for the formatter and for code actions later.

**Formatter (`esld fmt`).** Canonical style: 2-space indent, one statement per line, `key = value;` alignment off (keep simple), blocks always braced. Style is *the* canonical serialization used by round-trip tests. `--check` mode diffs without writing (CI-enforced).

**[Rust]** No parser generator — the grammar has ~30 productions; hand-rolled gives best spans, recovery, and trivia control. A tree-sitter grammar is generated alongside later for editor highlighting (M7).

### 5.2 `esld-ir` — model, expansion, JSON

Responsibilities:

1. **Link:** resolve every `type_ref` against built-ins + document `type` decls (+ vendor namespaces); resolve every node ref / port ref; produce E-101-style diagnostics for unknowns (feeds spec R-101/R-111 checks).
2. **Type instantiation:** flatten `extends` chains (cycle ⇒ error), merge `params` defaults with per-node property values.
3. **Expansion** (spec §9.3): circuits become protection node + optional controller node + cable node + edges; implicit bus sections materialize (`BOARD.bus`); `measures`/`ct`/`vt`/`trips` properties become signal/trip edges.
4. **Ser/de:** `serde` derive on the IR structs; the JSON shape is the normative one in spec §7.4. `export --format ir` emits it; `load-ir` accepts it (with `sourceSpan` optional) so external tools can skip parsing.

Include resolution: `syntax` exposes `include_resolver: Fn(&str) -> Result<SourceFile>`; the CLI supplies a path-rooted resolver (relative-to-includer, then `--include` search path, cycle detection via an active-stack). WASM supplies a JS callback — so browser builds can virtualize the filesystem.

### 5.3 `esld-check` — validation engine

- Rules are **data + pure functions**: `Rule { id, severity, applies(ctx) -> bool, run(ctx) -> Vec<Diagnostic> }`, registered in a static catalog. Severity may be overridden by code profiles (e.g. a plant standard downgrades R-307 to warning) — the *catalog* stays frozen; only presentation changes.
- Phases mirror spec §15: structural (R-1xx) run on the linked IR before expansion outputs are needed; voltage/earthing (R-2xx), protection/rating (R-3xx), source/island (R-4xx) run after expansion. Scenario/state rules (R-5xx/R-6xx) call into `esld-solve`.
- Code profiles: active `code` blocks merge into a `CodeParams` map; rules read typed keys with spec-declared defaults. Unknown keys are preserved (forward compat) but reported as notes.
- Quantity math: compare only after unit normalization (spec §4.6). Dimension mismatches (`rating_a = 230V`) are E-level diagnostics at IR build.

### 5.4 `esld-solve` — states, interlocks, scenarios

Evaluation of one scenario (spec §10.7, §13, §14):

```
fn evaluate(doc: &Ir, state: Option<&State>, sets: &[Set]) -> Solution
```

1. Start from state positions (chained `use`), default positions by type, then apply scenario `set`s.
2. Source availability: `online`/`offline`/`standby` semantics per spec §10.4; transfer devices select first available input by `priority`.
3. Conduction: union-find over closed switching devices and bus ties ⇒ **merged groups**; energized set = groups containing an available source, minus devices upstream-open.
4. Interlocks: evaluate boolean expressions against final positions; violated ⇒ R-603.
5. Expectations: `energized`/`de_energized`/`islanded` are set queries; `power_at` yields sign-only values (positive along `->`) unless bounded by a declared limit; `violation("R-x")` re-runs the named rule.
6. Islands: connected components w.r.t. sources; `islanded(X)` true iff X's component contains an island-capable source and no grid-tie source.

The solver is reachability + constraint logic only — this keeps it deterministic and fast and matches the spec's v0.1 scope. (`simulate` numerics are spec roadmap 0.5; the `Solution` struct is the extension point.)

### 5.5 `esld-layout` — deterministic layout

- **Algorithm:** layered (Sugiyama-family) with fixed tie-breaking:
  1. Rank by longest-path from sources (`rank = source_to_load` default; `flow` sets orientation).
  2. Order within ranks: stable sort by (board membership, section index, phase, tag). Boards are laid out as grouped clusters — a board's circuits share ranks inside the board's rectangle.
  3. Coordinates: integer grid in abstract units, `spacing` hint scales it; `column`/`busbar`/`orientation` hints override positions when satisfiable.
- Determinism: no iterative optimization with unstable convergence; every ordering key ends in `tag`, which is unique ⇒ total order ⇒ identical output.
- Output: `Layout { nodes: Map<tag, Rect+Anchor>, edges: EdgeRoute[] }` consumed by render; also exported with `export --format layout-json` for external renderers (debug + third-party use).

### 5.6 `esld-render` — SVG and symbols

- SVG written via a small string-builder (no DOM dependency; works in core ⇒ same code native and WASM). Escaping is mandatory for all text.
- **Symbol library as data:** each symbol is a small vector primitive set (lines/rects/arcs) keyed by `(type, option-set)`; two registries: `IEC` (IEC 60617-derived) and `ANSI` (IEEE 315-derived). Symbols are drawn from scratch as primitives — **no imported/scanned artwork** (licensing + determinism). Unit tests render every symbol alone and hash it.
- Board = labelled rectangle, sections = distinct bars joined through tie devices, protection between bar and circuit, measurement/trip links dotted/dashed per spec §16.2. Annotations (rating, curve, kA) as text near devices.
- Determinism check: same input ⇒ identical SVG bytes (snapshot hash after float normalization: fixed 2-decimal output).

### 5.7 `esld-cli`

```
esld 0.1.0 — Electrical Single Line Diagram toolchain

USAGE:
  esld <COMMAND> [OPTIONS] <FILE...>

COMMANDS:
  check    Parse + validate; report diagnostics            [P1]
  fmt      Canonical formatting (round-trip safe)          [P1]
  export   Emit IR JSON | DOT | CSV schedules | layout     [P1]
  render   Deterministic SVG drawing                       [P2]
  solve    Run states/interlocks/scenarios                 [P3]
  lint-lite Alias: check with --strict (no warnings)       [P1]
  completions  Shell completions                            [P1]
  lsp      Language server (later)                          [P5]

GLOBAL OPTIONS:
  --format human|json        Diagnostic output format (default human)
  --profile <path>...        Extra code-profile packs
  --include <path>...        Include search path
  --lenient                  Skip profile major-version gate
  --no-color / --color

RENDER OPTIONS:
  -o, --out <file>           Default: <input>.svg
  --symbols iec|ansi         Symbol registry (default iec)
  --paper a3|a4|letter|auto
  --theme light|dark

EXIT CODES:
  0  success, no error-severity diagnostics
  1  diagnostics contained at least one error
  2  usage / IO error
  3  internal error (bug; please report)
```

Human diagnostics follow compiler convention:

```
plant.esld:88:3: error[R-306]: breaking capacity below prospective fault
  --> breaker CB_IA: breaking_ka 25kA < fault 250MVA/~13.1kA (margin 20%)
  note: declared at plant.esld:12:9 (voltsys MV fault_mva)
```

JSON diagnostics (stable schema, consumed by CI and the WASM layer alike):

```json
{
  "file": "plant.esld",
  "diagnostics": [
    { "code": "R-306", "severity": "error",
      "message": "breaking capacity below prospective fault",
      "span": { "startLine": 88, "startCol": 3, "endLine": 88, "endCol": 44 },
      "notes": ["declared at plant.esld:12:9 (voltsys MV fault_mva)"] }
  ]
}
```

### 5.8 `esld-wasm` — JS/WASM target

**[Rust]** `wasm32-unknown-unknown`, `wasm-bindgen` + `serde-wasm-bindgen`. Exports (TS surface shipped in the npm package):

```ts
// @esld/core — generated from esld-wasm
export class Esld {
  static parse(text: string, opts?: ParseOptions): Esld;   // throws EsldError
  check(): Diagnostic[];               // JSON-shaped, same schema as CLI
  fmt(): string;
  exportIr(): unknown;                 // IR JSON value
  exportDot(): string;
  exportCsv(kind: "circuits" | "boards"): string;
  render(opts?: RenderOptions): string;                    // SVG
  solve(scenario?: string, state?: string): Solution;
}

export interface EsldError { diagnostics: Diagnostic[]; }
```

Usage sketches:

```html
<!-- browser -->
<script type="module">
  import { Esld } from "@esld/core";
  const src = await (await fetch("plant.esld")).text();
  const doc = Esld.parse(src);
  document.body.innerHTML = doc.render({ symbols: "iec", paper: "a3" });
</script>
```

```ts
// Node / Deno / edge
import { Esld } from "@esld/core";
import { readFileSync } from "node:fs";
const doc = Esld.parse(readFileSync("plant.esld", "utf8"));
const diags = doc.check();
if (diags.some(d => d.severity === "error")) { console.error(diags); process.exit(1); }
```

Constraints & budgets:

- Bundle ≤ **1.5 MB gzipped** (assert in CI; `opt-level = "z"`, `panic = "abort"`, LTO, strip; measure with `wasm-pack`).
- Parse+check ≤ **100 ms** for a 500-node document on a mid-range 2020 laptop (browser + native parity within 2.5×).
- No filesystem: includes resolve via a user-supplied callback (`ParseOptions.includeResolver`).
- Errors cross the boundary as values (`EsldError`), never as traps; `Result<>` everywhere at the boundary.
- Also publish a **`wasm32-wasi` build of the full CLI** as a bonus artifact (runnable under `wasmer`/`wasmtime`/Node-WASI for sandboxed CI) — zero extra code, CI-only packaging.

---

## 6. Conformance corpus and testing

The corpus is the contract between spec, CLI, and any second implementation. **One runner, all languages** — the runner is a small JSON-driven harness whose reference behavior is defined by fixtures, so it can be re-implemented in TS trivially.

```
corpus/
  valid/plant-main-tie-main.esld          → expect: zero errors
  invalid/r-306-low-breaking.esld         → expect: contains R-306 at line N
  invalid/r-410-parallel-no-sync.esld
  invalid/r-602-earth-switch-closed.esld
  roundtrip/comments-and-unknown-props.esld
  render/small-industrial.esld            → golden: small-industrial.svg.hash
  solve/grid-loss-ats.esld                → scenario expectations inline (spec §14)
```

Fixture sidecars (`.expected.json`) hold diagnostics as `code + severity + line`; messages are matched by prefix only (messages are not frozen).

Test layers:

| Layer | What | Tooling |
|---|---|---|
| Unit | lexer/parser table tests, quantity normalization, unit math | `cargo test` |
| Golden | corpus valid/invalid, render SVG hashes (normalized floats) | runner in CI |
| Round-trip property | `fmt(parse(x))` idempotent; AST-equivalence after cycle | `proptest` |
| Determinism | render twice, byte-compare; also across platforms | CI matrix |
| Fuzz | lexer/parser must not panic; any panic is a bug | `cargo-fuzz` targets `lex`, `parse`, `fmt` |
| Spec examples | every example in esld-spec.md is extracted (fenced blocks tagged `esld`) and must at least parse | doc-extraction script in CI |

CI matrix: Linux/macOS/Windows (native), plus `wasm32-unknown-unknown` tested under Node (`wasm-bindgen-test`).

---

## 7. Performance budgets (P4 gates)

| Operation | Budget (native / wasm) |
|---|---|
| Parse 500-node, 60-circuit document | ≤ 30 ms / ≤ 75 ms |
| Full check (all rules) on same | ≤ 50 ms / ≤ 125 ms |
| Solve one scenario | ≤ 20 ms / ≤ 50 ms |
| Render SVG (60 circuits) | ≤ 80 ms / ≤ 200 ms |
| Output SVG size (60 circuits) | ≤ 400 KB |
| WASM bundle | ≤ 1.5 MB gzip |
| Cold init (wasm instantiate) | ≤ 10 ms |

Measured with `criterion` benches against a generated 500/2000-node corpus; budgets enforced in CI (fail builds on regression > 20%).

---

## 8. Security

- Parsing runs on **untrusted input** by design (fuzzed from day one; no `unsafe` outside audited spots; integer-overflow checks on).
- Include resolver is sandboxed by the host: the CLI restricts includes to `--include` roots + including-file directory (no network, no `..` escape); WASM defers entirely to the JS callback.
- SVG output escapes all text (anti-XSS for embedded diagrams); no external references, fonts, or scripts in emitted SVG.
- No telemetry, no network in any component.

---

## 9. Versioning and distribution

- **Spec and tool version independently.** Tool `--version` reports both (`esld 0.4.1 / spec esld/1.0 (draft)`).
- Semver for the tool; the JSON diagnostic schema and IR shape are covered by the tool's semver from 1.0, and by a `--strict` stability flag before that.
- Native: GitHub Releases with static binaries (x86_64/aarch64, musl), `cargo install esld-cli`, Homebrew tap later.
- JS: npm `@esld/core` (ESM + CJS + `.d.ts`), CDN build for `<script>` usage, semver-matched to the tool.
- Corpus versions with the spec; each corpus case records `minSpecVersion`.

---

## 10. Milestones

| ID | Scope | Acceptance criteria |
|---|---|---|
| M0 | Repo, CI matrix, crate skeletons, lexer with golden tests | All tokens of every spec example lex correctly |
| M1 | Full parser + AST + `fmt` | Round-trip property passes on corpus; spec examples extracted and parsed |
| M2 | IR: link, type instantiation, expansion, JSON export | `export ir` on worked examples matches reviewed golden JSON |
| M3 | `check`: R-1xx + R-2xx, human+JSON diagnostics | invalid/ corpus green for structural + voltage rules |
| M4 | `check`: R-3xx/R-4xx + code profiles | invalid/ corpus green for protection + source rules |
| M5 | `render`: layout + SVG, IEC subset | Deterministic golden SVGs across 3 OSes; 60-circuit budget met |
| M6 | `solve`: states, interlocks, scenarios; R-5xx/R-6xx | Both spec worked examples produce their documented expectations |
| M7 | WASM + npm + minimal playground page | Browser round-trip < 100 ms; bundle ≤ 1.5 MB gz |
| M8 | PDF export, ANSI symbol set, tree-sitter grammar + LSP | Spec §16.2 renderer contract fully green |

Each milestone lands with its corpus cases and a tagged release (`v0.1.0-m1` style pre-releases until P1 completes).

---

## 11. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Layout quality below hand-drawn norms | Tool perceived as toy | Layered-with-hints first, publish layout-json so third parties can compete on rendering; hints (spec §16.1) are the escape hatch |
| Symbol fidelity vs licensing (IEC/ANSI standards are copyrighted documents) | Legal exposure | Draw primitives from scratch informed by the standards' conventions; never embed artwork/scans; document provenance per symbol |
| Spec churn during 0.x | Rework | Profile-gate (`esld/1.0` header) from M1; corpus versions with spec; breaking changes bump profile major |
| WASM bundle creep | Web target unusable | Size budget in CI from M7, `opt-level=z`, no heavyweight deps (no `chrono`, no `regex` unless justified) |
| Two-implementation drift (if TS port happens) | Invalid conformance claims | Corpus is the single source of truth; conformance runner spec'd in §6; CI badge per implementation |
| Quantity/unit edge cases (`mm²`, `µ`, `2.5mm2` vs `2.5 mm2`) | Subtle validation bugs | Property tests over unit normalization table; normalize at IR boundary, never in rules |

---

## 12. Immediate next steps

1. Land this plan + spec (this repo, `Design/`).
2. M0 skeleton PR: workspace, CI, lexer, first golden tests — including extraction of every fenced `esld` block from the spec as test inputs.
3. Draft the IR JSON Schema (spec roadmap 0.4) early instead of late — it is the cheapest contract to review and unblocks third-party tooling before the tool exists.
4. Choose the symbol set scope for M5: proposal — 24 symbols covering the built-in type library's IEC renderings (breaker, disconnector, fuse, transformer 2w, CT, VT, relay, motor, generator, grid, PV, battery, inverter, UPS, earth, NGR, bus/section, board, ATS, meter, capacitor, reactor, cable marker, SPD).
