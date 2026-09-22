# BusBar — ESLD Toolchain — Implementation Plan v0.1

**Status:** Draft for discussion
**Working name:** **BusBar** — the toolchain (repo `github.com/baradhili/busbar`). The *language* it speaks remains **ESLD**, per the companion spec.
**Decided:** implementation language is **Rust**; acceptance and conformance tests are written in **Cucumber (Gherkin)** and executed with rust-cucumber.
**Companion to:** [esld-spec.md](esld-spec.md) — ESLD v0.1 language specification
**Scope:** Plan for a reference toolchain implementing the Reader, Validator, Renderer, and Solver conformance classes. **CLI first**, with the same core delivered to **JavaScript/WASM** as a first-class target.

---

## 1. Purpose and constraints

Build a tool that:

1. Parses, validates, formats, renders, and solves `.esld` documents per the v0.1 spec.
2. Ships first as a **native CLI** (`busbar`) — single static binary, no runtime deps.
3. Ships the **same core** as a **WASM module** consumable from JavaScript/TypeScript (browser, Node, Deno, edge runtimes), because the natural home of an SLD renderer is the web (viewers, editor integrations, docs sites, review tools).
4. Is deterministic: identical input + options ⇒ identical output (bit-for-bit), per spec §16.2.
5. Is testable against a shared, implementation-neutral **conformance suite** written in **Cucumber (Gherkin)**, so that any future second implementation can be certified against the same scenarios.
6. Is implemented in **Rust** — decided; §3 records the decision and the rationale.

### 1.1 Non-goals for v0.1 of the tool

- No GUI editor (the WASM target enables one later; we do not build it).
- No LSP server until M8 (grammar must stabilize first).
- No numeric power flow (spec roadmap 0.5).
- No PDF generation natively in M1–M6; SVG is the normative rendering. PDF arrives via a dedicated pipeline in M8.

---

## 2. Delivery strategy

| Phase  | Deliverable                                                                     | Gate                                             |
| ------ | ------------------------------------------------------------------------------- | ------------------------------------------------ |
| **P1** | Native CLI: parse → IR → validate → export (JSON/DOT/CSV), `fmt`                | All R-1xx/R-2xx rules green on corpus            |
| **P2** | Deterministic SVG renderer (IEC symbol subset), `render`                        | Golden SVG snapshots stable across platforms     |
| **P3** | Solver: states, interlocks, scenarios; R-5xx/R-6xx                              | Spec worked examples produce expected results    |
| **P4** | Create per Board physical layouts using manufacturer or generic device drawings | Correct SVG snapshots across all platforms       |
| **P5** | WASM module + npm package + web playground                                      | Round-trips in browser < 100 ms for 500-node doc |
| **P6** | R-3xx/R-4xx full, code profiles, PDF, LSP                                       | Conformance suite ≥ 40 documents                 |

The order is deliberate: everything downstream of parsing (validation, rendering, solving) is pure computation over the IR, so the CLI and WASM targets share 100% of the interesting code and differ only in the shell (argv/stdin/stdout vs. JS bindings).

---

## 3. Technology decisions (recorded)

### 3.1 Decisions

| Decision                         | Choice                                                                   |
| -------------------------------- | ------------------------------------------------------------------------ |
| Implementation language          | **Rust** — all crates, both shells                                       |
| Acceptance & conformance testing | **Cucumber (Gherkin)** via rust-cucumber (§6)                            |
| Native shell                     | `busbar` binary (`clap`)                                                 |
| Web shell                        | `wasm-bindgen` module published as `@busbar/core` (npm name provisional) |

### 3.2 Why Rust (rationale retained)

- **One codebase, two targets.** `wasm32-unknown-unknown` + `wasm-bindgen` is the mature path to a JS-callable core, while the same crates build a static musl CLI binary.
- **Determinism control.** `BTreeMap`/`IndexMap`, no randomized map iteration, one fixed float formatter — the spec's bit-stable rendering contract (spec §16.2) is straightforward to guarantee.
- **Typed JSON IR.** `serde` gives the normative IR shape (spec §7.4) with round-trip tests nearly for free.
- **Parser ergonomics.** A ~30-production grammar hand-rolled with spans and trivia is well within Rust's comfort zone.
- Go was the runner-up (great CLI story) but ships larger WASM binaries with weaker JS interop. TypeScript was the early alternative; it remains possible *later* as a port certified against the same Gherkin scenarios — not as the primary implementation.

### 3.3 Core/shell split

- `busbar-cli` — native binary.
- `busbar-wasm` — JS/WASM bindings.

The core crates (`syntax`, `ir`, `check`, `solve`, `layout`, `render`) have **no I/O and no wall-clock dependency**: `no_std`-friendly where practical, `std` + `serde` in practice. This is what makes the CLI/WASM split cheap and is a hard architectural rule (§4.3).

---

## 4. System architecture

### 4.1 Repository layout

```
SLD/ (repo "busbar")
├── Design/                     # specs (this document)
├── crates/
│   ├── busbar-syntax/          # lexer, parser, AST, trivia, formatter
│   ├── busbar-ir/              # model, expansion, JSON (de)serialization
│   ├── busbar-check/           # rule engine, diagnostics, code profiles
│   ├── busbar-solve/           # states, interlocks, scenarios, reachability
│   ├── busbar-layout/          # deterministic layout solver
│   ├── busbar-render/          # SVG writer + symbol library data
│   ├── busbar-cli/             # `busbar` native binary
│   └── busbar-wasm/            # wasm-bindgen bindings (@busbar/core)
├── corpus/                     # .esld fixtures referenced by features
│   ├── valid/                  # parse + validate clean
│   ├── invalid/                # expected diagnostics, keyed by rule ID
│   ├── roundtrip/              # fmt(parse(x)) == fmt(parse(fmt(parse(x))))
│   ├── render/                 # golden SVG hashes
│   └── solve/                  # scenario expectation fixtures
├── features/                   # Gherkin features + steps.rs (Cucumber, §6)
│   ├── parsing.feature
│   ├── rules-structural.feature        # R-1xx
│   ├── rules-voltage-earthing.feature  # R-2xx
│   ├── rules-protection.feature        # R-3xx
│   ├── rules-sources-islands.feature   # R-4xx
│   ├── rules-scenarios.feature         # R-5xx
│   ├── rules-states-interlocks.feature # R-6xx
│   ├── roundtrip.feature
│   ├── render.feature
│   ├── solve.feature
│   └── spec-examples.feature
└── tools/                      # spec-example extractor, snapshot normalizer
```

Note: the seed samples formerly at `tests/sample1.md` / `tests/sample2.md` moved to `corpus/valid/sample1.esld` / `sample2.esld` when the workspace landed; the Cucumber step definitions live at `features/steps.rs`, co-located with the Gherkin files.

### 4.2 Pipeline

```
             text (.esld)
                │
        ┌───────▼───────┐
        │ busbar-syntax │  lexer → parser → AST (+ trivia, spans)
        └───────┬───────┘
                │ AST
        ┌───────▼───────┐   type defs      ┌─────────────┐
        │   busbar-ir   │◄─(built-ins + ───►│ vendor packs│
        │  link/expand  │    user types)    └─────────────┘
        └───────┬───────┘
                │ IR (Document)
        ┌───────▼───────┐
        │ busbar-check  │  rule engine → diagnostics[]
        └───────┬───────┘
        ┌───────▼───────┐
        │ busbar-solve  │  states/interlocks/scenarios → results
        └───────┬───────┘
        ┌───────▼───────┐
        │ busbar-layout │  ranked graph → geometry (abstract units)
        └───────┬───────┘
        ┌───────▼───────┐
        │ busbar-render │  geometry + symbols → SVG string
        └───────────────┘
```

Each stage is a pure function of its input plus options. No stage writes to disk or clock.

### 4.3 Hard rules

1. **Core purity.** `syntax`…`render` MUST NOT: touch the filesystem (except `syntax` receiving an include-resolver *callback*), read env vars, use time, randomness, threads, or unordered iteration on anything that reaches output.
2. **Determinism.** Use `BTreeMap`/`IndexMap` (deterministic) — never `HashMap` — for anything serialized or rendered; sort by tag where the spec doesn't order; all numbers go through one fixed formatter.
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

Codes: `R-*` are spec rule IDs. `E-*` are tool errors (lex/parse/link/internal). `W-*` tool warnings. The catalog of `E-*` codes lives in `busbar-syntax`/`busbar-ir` docs and is frozen per release.

---

## 5. Component specifications

### 5.1 `busbar-syntax` — lexer, parser, formatter

**Lexer.** Hand-written, span-tracking. Notable decisions:

- **Quantities:** lex `NUMBER` and trailing `UNIT` separately, then *glue* in the parser (`spaced` quantities like `230 V` allowed by spec §4.6). Unit tokenization must not eat identifiers: `400A` → `NUMBER(400) UNIT(A)`; `fault_mva` → `IDENT`.
- **Reserved words** per spec §4.8, contextual after `.` for member names (`GRID.out`).
- Comments and whitespace attach as **trivia** to the following token (leading) / preceding (trailing), preserving round-trip.

**Parser.** Recursive descent following the EBNF one-to-one (the grammar is LL-ish; the only soft spot is `voltsys` positional vs block bodies and `value` vs `ref` in property position — both resolved with one-token lookahead).

- Error recovery: synchronize on `;` and `}` at statement level so one typo yields one diagnostic, not forty.
- AST nodes carry `Span`s and trivia ranges; a `CstView` maps AST ↔ source for the formatter and for code actions later.

**Formatter (`esld fmt`).** Canonical style: 2-space indent, one statement per line, `key = value;` alignment off (keep simple), blocks always braced. Style is *the* canonical serialization used by round-trip tests. `--check` mode diffs without writing (CI-enforced).

No parser generator — the grammar has ~30 productions; hand-rolled gives best spans, recovery, and trivia control. A tree-sitter grammar is generated alongside later for editor highlighting (M8).

### 5.2 `busbar-ir` — model, expansion, JSON

Responsibilities:

1. **Link:** resolve every `type_ref` against built-ins + document `type` decls (+ vendor namespaces); resolve every node ref / port ref; produce E-101-style diagnostics for unknowns (feeds spec R-101/R-111 checks).
2. **Type instantiation:** flatten `extends` chains (cycle ⇒ error), merge `params` defaults with per-node property values.
3. **Expansion** (spec §9.3): circuits become protection node + optional controller node + cable node + edges; implicit bus sections materialize (`BOARD.bus`); `measures`/`ct`/`vt`/`trips` properties become signal/trip edges.
4. **Ser/de:** `serde` derive on the IR structs; the JSON shape is the normative one in spec §7.4. `export --format ir` emits it; `load-ir` accepts it (with `sourceSpan` optional) so external tools can skip parsing.

Include resolution: `syntax` exposes `include_resolver: Fn(&str) -> Result<SourceFile>`; the CLI supplies a path-rooted resolver (relative-to-includer, then `--include` search path, cycle detection via an active-stack). WASM supplies a JS callback — so browser builds can virtualize the filesystem.

### 5.3 `busbar-check` — validation engine

- Rules are **data + pure functions**: `Rule { id, severity, applies(ctx) -> bool, run(ctx) -> Vec<Diagnostic> }`, registered in a static catalog. Severity may be overridden by code profiles (e.g. a plant standard downgrades R-307 to warning) — the *catalog* stays frozen; only presentation changes.
- Phases mirror spec §15: structural (R-1xx) run on the linked IR before expansion outputs are needed; voltage/earthing (R-2xx), protection/rating (R-3xx), source/island (R-4xx) run after expansion. Scenario/state rules (R-5xx/R-6xx) call into `busbar-solve`.
- Code profiles: active `code` blocks merge into a `CodeParams` map; rules read typed keys with spec-declared defaults. Unknown keys are preserved (forward compat) but reported as notes.
- Quantity math: compare only after unit normalization (spec §4.6). Dimension mismatches (`rating_a = 230V`) are E-level diagnostics at IR build.

### 5.4 `busbar-solve` — states, interlocks, scenarios

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

### 5.5 `busbar-layout` — deterministic layout

> Drawing conventions are codified in `Design/layout-guidance.md`
> (distilled from the reference corpus in `Design/refs/`); the engine
> implements its "adopted now" table and the invariant suite
> machine-checks it.

- **Algorithm:** layered (Sugiyama-family) with fixed tie-breaking:
  1. Rank by longest-path from sources (`rank = source_to_load` default; `flow` sets orientation).
  2. Order within ranks: stable sort by (board membership, section index, phase, tag). Boards are laid out as grouped clusters — a board's circuits share ranks inside the board's rectangle.
  3. Coordinates: integer grid in abstract units, `spacing` hint scales it; `column`/`busbar`/`orientation` hints override positions when satisfiable.
- Determinism: no iterative optimization with unstable convergence; every ordering key ends in `tag`, which is unique ⇒ total order ⇒ identical output.
- Output: `Layout { nodes: Map<tag, Rect+Anchor>, edges: EdgeRoute[] }` consumed by render; also exported with `export --format layout-json` for external renderers (debug + third-party use).

### 5.6 `busbar-render` — SVG and symbols

- SVG written via a small string-builder (no DOM dependency; works in core ⇒ same code native and WASM). Escaping is mandatory for all text.
- **Symbol library as data:** each symbol is a small vector primitive set (lines/rects/arcs) keyed by `(type, option-set)`; two registries: `IEC` (IEC 60617-derived) and `ANSI` (IEEE 315-derived). Symbols are drawn from scratch as primitives — **no imported/scanned artwork** (licensing + determinism). Unit tests render every symbol alone and hash it.
- Board = labelled rectangle, sections = distinct bars joined through tie devices, protection between bar and circuit, measurement/trip links dotted/dashed per spec §16.2. Annotations (rating, curve, kA) as text near devices.
- Determinism check: same input ⇒ identical SVG bytes (snapshot hash after float normalization: fixed 2-decimal output).

### 5.7 `busbar-cli`

```
busbar 0.1.0 — Electrical Single Line Diagram toolchain (ESLD)

USAGE:
  busbar <COMMAND> [OPTIONS] <FILE...>

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

### 5.8 `busbar-wasm` — JS/WASM target

`wasm32-unknown-unknown`, `wasm-bindgen` + `serde-wasm-bindgen`. Exports (TS surface shipped in the npm package `@busbar/core`):

```ts
// @busbar/core — generated from busbar-wasm
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
  import { Esld } from "@busbar/core";
  const src = await (await fetch("plant.esld")).text();
  const doc = Esld.parse(src);
  document.body.innerHTML = doc.render({ symbols: "iec", paper: "a3" });
</script>
```

```ts
// Node / Deno / edge
import { Esld } from "@busbar/core";
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

## 6. Testing — Cucumber (Gherkin) as the acceptance contract

Acceptance and conformance testing is written in **Cucumber**: features in Gherkin, step definitions in Rust, executed with [rust-cucumber](https://crates.io/crates/cucumber). The same `.feature` files run against the native build and (tagged `@wasm`) against the WASM build under Node, so the scenarios double as cross-target conformance checks and as the executable contract for any future second implementation.

### 6.1 Wiring

- Crate: `cucumber` (rust-cucumber), async steps on `tokio`.
- Feature files and step definitions co-located in `features/` (`steps.rs` is a `harness = false` test target), run as `cargo test --test cucumber` — no special binary in CI.
- A `World` struct carries state between steps (source text → parsed document → diagnostics / render output / solve result).
- Tags select subsets: `@wasm`, `@render`, `@slow`.

### 6.2 Corpus and features

Corpus files stay plain `.esld` data under `corpus/`; Gherkin Scenario Outlines drive them:

```gherkin
Feature: Structural validation rules (R-1xx)
  Scenario Outline: Invalid documents report exactly the expected rule
    Given the document <file>
    When I check it
    Then it reports <code> at line <line>
    And it reports no other errors

    Examples:
      | file                              | code  | line |
      | invalid/r102-duplicate-tag.esld   | R-102 | 9    |
      | invalid/r110-unlisted-feed.esld   | R-110 | 14   |
      | invalid/r113-no-bus-on-split.esld | R-113 | 21   |

  Scenario: Valid documents pass clean
    Given the document "valid/sample1.esld"
    When I check it
    Then there are no error diagnostics
```

```gherkin
Feature: Round-trip stability
  Scenario: Formatting is idempotent and semantics-preserving
    Given the document <file>
    When I format it
    And I format the result again
    Then both formatted outputs are identical
    And parsing the formatted output yields an AST equal to the original's
```

```gherkin
Feature: Rendering determinism
  Scenario: Identical input renders identical SVG
    Given the document <file>
    When I render it with symbols "iec"
    Then the SVG hash is <hash>

  @wasm
  Scenario: The WASM build renders the same hash
    Given the document <file>
    When I render it with symbols "iec" on the WASM build
    Then the SVG hash is <hash>
```

```gherkin
Feature: Solver scenarios
  Scenario Outline: Scenario expectations from the spec examples
    Given the document <file>
    When I solve scenario <scenario>
    Then all expectations hold

    Examples:
      | file                 | scenario                        |
      | solve/grid-loss.esld | Grid loss at night, battery 90% |
```

```gherkin
Feature: Spec examples are valid
  Scenario: Every fenced esld block in the spec at least parses
    Given the spec "Design/esld-spec.md"
    When I extract its fenced example blocks
    Then each parses without error
    And the worked examples (§18) check clean
```

### 6.3 Test layers

| Layer                    | What                                                   | Tooling                                                               |
| ------------------------ | ------------------------------------------------------ | --------------------------------------------------------------------- |
| Unit                     | lexer/parser tables, quantity normalization, unit math | `cargo test`                                                          |
| Acceptance / conformance | corpus cases as Gherkin scenarios                      | **Cucumber** (rust-cucumber)                                          |
| Round-trip property      | `fmt(parse(x))` idempotent, AST-equivalence            | `proptest` in-crate; surfaced as a Cucumber scenario for corpus files |
| Determinism              | render twice, byte-compare; features re-run per OS     | Cucumber + CI matrix                                                  |
| Fuzz                     | lexer/parser must not panic                            | `cargo-fuzz` targets `lex`, `parse`, `fmt`                            |

CI matrix: Linux/macOS/Windows run the full feature suite natively; one Node job re-runs `@wasm`-tagged scenarios against the WASM build.

### 6.4 Fixture provenance

`corpus/valid/sample1.esld` and `corpus/valid/sample2.esld` are the converted seed examples (formerly `tests/sample1.md` / `tests/sample2.md`; moved and renamed when the workspace landed). Every other case is authored against the rule it exercises — one minimal document per rule ID where practical.

---

## 7. Performance budgets (P4 gates)

| Operation                           | Budget (native / wasm) |
| ----------------------------------- | ---------------------- |
| Parse 500-node, 60-circuit document | ≤ 30 ms / ≤ 75 ms      |
| Full check (all rules) on same      | ≤ 50 ms / ≤ 125 ms     |
| Solve one scenario                  | ≤ 20 ms / ≤ 50 ms      |
| Render SVG (60 circuits)            | ≤ 80 ms / ≤ 200 ms     |
| Output SVG size (60 circuits)       | ≤ 400 KB               |
| WASM bundle                         | ≤ 1.5 MB gzip          |
| Cold init (wasm instantiate)        | ≤ 10 ms                |

Measured with `criterion` benches against a generated 500/2000-node corpus; budgets enforced in CI (fail builds on regression > 20%).

---

## 8. Security

- Parsing runs on **untrusted input** by design (fuzzed from day one; no `unsafe` outside audited spots; integer-overflow checks on).
- Include resolver is sandboxed by the host: the CLI restricts includes to `--include` roots + including-file directory (no network, no `..` escape); WASM defers entirely to the JS callback.
- SVG output escapes all text (anti-XSS for embedded diagrams); no external references, fonts, or scripts in emitted SVG.
- No telemetry, no network in any component.

---

## 9. Versioning and distribution

- **Spec and tool version independently.** Tool `--version` reports both (`busbar 0.4.1 / spec esld/1.0 (draft)`).
- Semver for the tool; the JSON diagnostic schema and IR shape are covered by the tool's semver from 1.0, and by a `--strict` stability flag before that.
- Native: GitHub Releases with static binaries (x86_64/aarch64, musl), `cargo install busbar-cli`, Homebrew tap later.
- JS: npm `@busbar/core` (name provisional; ESM + CJS + `.d.ts`), CDN build for `<script>` usage, semver-matched to the tool.
- Corpus versions with the spec; each corpus case records `minSpecVersion`.

---

## 10. Milestones

| ID  | Scope                                                      | Acceptance criteria                                                                |
| --- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| M0  | Repo, CI matrix, crate skeletons, Cucumber scaffold, lexer | First `.feature` green; every spec example lexes correctly                         |
| M1  | Full parser + AST + `fmt`                                  | `roundtrip.feature` green; `spec-examples.feature` parses every block              |
| M2  | IR: link, type instantiation, expansion, JSON export       | `export ir` on worked examples matches reviewed golden JSON                        |
| M3  | `check`: R-1xx + R-2xx, human+JSON diagnostics             | `rules-structural.feature` + `rules-voltage-earthing.feature` green                |
| M4  | `check`: R-3xx/R-4xx + code profiles                       | `rules-protection.feature` + `rules-sources-islands.feature` green                 |
| M5  | `render`: layout + SVG, IEC subset                         | `render.feature` hashes stable across 3 OSes; 60-circuit budget met                |
| M6  | `solve`: states, interlocks, scenarios; R-5xx/R-6xx        | `solve.feature` reproduces both spec worked examples' expectations                 |
| M7  | WASM + npm + minimal playground page                       | Browser round-trip < 100 ms; bundle ≤ 1.5 MB gz; `@wasm` scenarios pass under Node |
| M8  | PDF export, ANSI symbol set, tree-sitter grammar + LSP     | Spec §16.2 renderer contract fully green                                           |

Each milestone lands with its corpus cases and a tagged release (`v0.1.0-m1` style pre-releases until P1 completes).

---

## 11. Risks and mitigations

| Risk                                                                        | Impact                     | Mitigation                                                                                                                       |
| --------------------------------------------------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Layout quality below hand-drawn norms                                       | Tool perceived as toy      | Layered-with-hints first, publish layout-json so third parties can compete on rendering; hints (spec §16.1) are the escape hatch |
| Symbol fidelity vs licensing (IEC/ANSI standards are copyrighted documents) | Legal exposure             | Draw primitives from scratch informed by the standards' conventions; never embed artwork/scans; document provenance per symbol   |
| Spec churn during 0.x                                                       | Rework                     | Profile-gate (`esld/1.0` header) from M1; corpus versions with spec; breaking changes bump profile major                         |
| WASM bundle creep                                                           | Web target unusable        | Size budget in CI from M7, `opt-level=z`, no heavyweight deps (no `chrono`, no `regex` unless justified)                         |
| Future second implementation drifts (e.g. a TS port)                        | Invalid conformance claims | Gherkin features are the shared contract — any implementation runs the same `.feature` files; CI badge per implementation        |
| Gherkin suite bloat / slow scenarios                                        | CI friction                | Scenario Outlines + Examples tables keep step code small; `@slow` excluded from PR-triggered runs                                |
| Quantity/unit edge cases (`mm²`, `µ`, `2.5mm2` vs `2.5 mm2`)                | Subtle validation bugs     | Property tests over unit normalization table; normalize at IR boundary, never in rules                                           |

---

## 12. Immediate next steps

1. Land this plan + spec (this repo, `Design/`).
2. M0 skeleton PR: Cargo workspace, CI, Cucumber scaffold with the first `.feature` files, and the lexer — including extraction of every fenced `esld` block from the spec as feature inputs; the samples moved from `tests/` to `corpus/valid/*.esld`.
3. Draft the IR JSON Schema (spec roadmap 0.4) early instead of late — it is the cheapest contract to review and unblocks third-party tooling before the tool exists.
4. Choose the symbol set scope for M5: proposal — 24 symbols covering the built-in type library's IEC renderings (breaker, disconnector, fuse, transformer 2w, CT, VT, relay, motor, generator, grid, PV, battery, inverter, UPS, earth, NGR, bus/section, board, ATS, meter, capacitor, reactor, cable marker, SPD).
