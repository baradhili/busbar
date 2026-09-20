# AGENTS.md — working guide for this repository

Orientation for AI coding agents (and humans in a hurry). Read this before
making changes.

## What this is

**BusBar** is the Rust toolchain for **ESLD**, the Electrical Single Line
Diagram language.

- Language spec (normative): `Design/esld-spec.md`
- Implementation plan (architecture, milestones): `Design/esld-implementation.md`
- Status: **M0 scaffold** — crates are doc-only stubs; no ESLD logic exists yet.

Naming: the *tool* is BusBar; the *language* is ESLD. Don't rename either.

## Repo map

| Path | Contents |
|---|---|
| `crates/busbar-syntax` | Lexer, parser, AST, trivia, formatter (M1) |
| `crates/busbar-ir` | Linking, type instantiation, expansion, JSON IR (M2) |
| `crates/busbar-check` | Rule engine R-1xx..R-6xx, code profiles (M3/M4) |
| `crates/busbar-solve` | States, interlocks, scenarios (M6) |
| `crates/busbar-layout` | Deterministic layered layout (M5) |
| `crates/busbar-render` | SVG writer + symbol data (M5) |
| `crates/busbar-cli` | `busbar` binary (M1+) |
| `crates/busbar-wasm` | `@busbar/core` bindings (M7) |
| `features/` | Gherkin acceptance features (one file per spec area) |
| `features/steps.rs` | Cucumber step definitions (`harness = false` target) |
| `corpus/{valid,invalid,roundtrip,render,solve}` | `.esld` fixtures driven by features |
| `tools/` | Spec-example extractor etc. (future) |
| `Design/` | Specs — the source of truth |

## Commands

```
make build          # cargo build --workspace
make test           # cargo test --workspace (includes the Cucumber suite)
make cucumber       # cargo test --test cucumber   (features only)
make cucumber-incomplete # @incomplete scenarios — RED until milestones land
make lint           # cargo clippy --workspace --all-targets -- -D warnings
make fmt-check      # cargo fmt --all -- --check
make wasm           # cargo build -p busbar-wasm --target wasm32-unknown-unknown
```

Toolchain: stable, pinned via `rust-toolchain.toml` (rustfmt + clippy
included). CI runs the full gate on Linux/macOS/Windows plus a wasm32 build
job (`.github/workflows/ci.yml`).

### Test suite states

Implemented features (parsing, round-trip, R-1xx, R-2xx, rendering)
are green and run in the default gate. Scenarios for areas whose milestones have not
landed are tagged `@incomplete` and excluded from the default run and
CI — run them with `make cucumber-incomplete` (or
`BUSBAR_INCOMPLETE=1`); they fail honestly on undefined steps until
implemented, then lose the tag. When adding steps for a new area,
define them in `features/steps.rs` and untag its scenarios.
`features/README.md` maps every feature file to its design-doc section.

## Conventions

- **Conventional commits** (`docs:`, `build:`, `feat:`, `fix:`, `test:`),
  matching existing history.
- **No `unsafe`** anywhere — workspace lints forbid it.
- **Core crates are pure and deterministic**: no filesystem, environment,
  clock, randomness, or `HashMap` iteration reaching output; use
  `BTreeMap`/`IndexMap`. Full rules in the implementation plan §4.3.
- **Every rule or behavior change ships with a Gherkin scenario** (and a
  corpus fixture where applicable). Scenarios for unimplemented areas are
  tagged `@incomplete` in `features/` (excluded from the default run);
  remove the tag as milestones land.
- **Gherkin gotcha**: description lines must not *start with* Gherkin
  keywords (`Scenario(s)`, `Rule`, `Given`, `When`, `Then`, `Examples`, …) —
  the parser rejects the file. Keep such words mid-sentence.
- Diagnostics are values (`code`, `severity`, `span`), never panics or bare
  strings (implementation plan §4.4).
- **CI is debug-profile and test-only on every branch** while in fast dev:
  no `--release` builds, artifacts, or publishing — enforced by a policy
  step in `.github/workflows/ci.yml`.

## Before you commit

Local gate (the review of record — do not run external review CLIs
such as CodeRabbit; the maintainer reviews on the `coderabbit-fixes`
branch):

```
make lint && make fmt-check && make test
```

