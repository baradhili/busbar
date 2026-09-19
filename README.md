# Busbar

> Plain-text single-line diagrams for power systems.

Busbar is an open-source tool for writing electrical single-line diagrams (SLDs) as plain text, then validating, analyzing, and rendering them. It is designed for engineers who want diagrams that are diffable, reviewable, and automatable.

## Status

Early design. The DSL and CLI are not stable yet. Feedback and design contributions are welcome.

## Goals

- Keep SLDs in plain text and version control.
- Make electrical topology easy to review in pull requests.
- Validate common power-system mistakes before they reach a drawing.
- Render clean SVG/PNG/PDF diagrams from the same source.
- Expose analysis as JSON for scripts and CI.
- Provide a CLI and a library API.

## Non-goals

- Replacing full CAD/PLC design suites.
- Performing protection coordination studies.
- Modeling every electrical component on day one.

## Example

`example.bus`

```bus
system "Campus Substation"
voltage 13.8kV 3ph 60Hz

source grid "Utility Grid" {
  voltage: 13.8kV
  fault_current: 25kA
}

bus main "Main 13.8kV Bus" {
  voltage: 13.8kV
}

breaker b1 "Main Breaker" {
  from: grid
  to: main
  rating: 600A
}

transformer t1 "Transformer 1" {
  from: main
  to: lvbus
  rating: 2.5MVA
  voltage: 13.8kV / 480V
}

bus lvbus "480V Bus" {
  voltage: 480V
}

load motor m1 "Chiller" {
  from: lvbus
  hp: 100
  voltage: 480V
}
```

Render it:

```sh
busbar render example.bus -o example.svg
```

Analyze it:

```sh
busbar analyze example.bus
```

Example output:

```text
System: Campus Substation
  Buses:        2
  Sources:      1
  Breakers:     1
  Transformers: 1
  Loads:        1

Warnings:
  - m1: no protection device between lvbus and load
```

## Planned CLI

```sh
busbar validate <file>        # parse and validate
busbar analyze <file>         # connectivity, voltage, ratings, islands
busbar render <file>          # SVG by default
busbar render <file> -f png   # PNG/PDF later
busbar graph <file>           # export DOT/JSON graph
busbar fmt <file>             # format the DSL
busbar lint <file>            # opinionated checks
```

## DSL sketch

Busbar files use `.bus` (or `.sld`) and describe a system as components plus connections.

Top-level:

```bus
system "Name"
voltage 480V 3ph 60Hz
```

Components:

```bus
source <id> "Label" { ... }
bus <id> "Label" { ... }

breaker <id> "Label" {
  from: <id>
  to: <id>
  rating: 600A
}

transformer <id> "Label" {
  from: <id>
  to: <id>
  rating: 2.5MVA
  voltage: 13.8kV / 480V
}

load <id> "Label" {
  from: <id>
  hp: 100
  voltage: 480V
}

generator <id> "Label" { ... }

feeder <id> "Label" {
  from: <id>
  to: <id>
  rating: 400A
}
```

Common attributes:

- `from`, `to` — topology
- `voltage` — nominal voltage
- `rating` — current or power rating
- `fault_current` — short-circuit contribution
- `hp`, `kw`, `kva` — load/generation size
- `label`, `notes` — documentation

Units are parsed and checked where possible: `V`, `kV`, `A`, `kA`, `VA`, `kVA`, `MVA`, `hp`, `kW`, `MW`, `Hz`.

Components are intended to be order-independent, so an element can be referenced before it is defined.

## Analysis

Busbar aims to answer questions like:

- Is every load reachable from a source?
- Are there isolated buses or islands?
- Does voltage match across connected components?
- Is a breaker/feeder undersized for downstream load?
- What is the upstream path for a given load?
- What changes if a breaker is open?

Open/closed state is planned:

```bus
breaker b1 "Main Breaker" {
  from: grid
  to: main
  rating: 600A
  state: closed
}
```

## Rendering

Rendering is intended to be deterministic and script-friendly:

- SVG first
- Graphviz DOT export
- PNG/PDF via external renderer later
- Layout hints in the text file when automatic layout is not enough

## Install

Not yet published. Planned:

```sh
# placeholder
cargo install busbar
# or
pip install busbar
```

## Development

```sh
git clone https://github.com/OWNER/busbar.git
cd busbar
# build/test commands will be added with the first implementation
```

## Repository layout

```text
busbar/
  docs/           # language and analysis docs
  examples/       # example .bus files
  packages/       # core parser, analyzer, renderer, CLI
  tests/          # fixtures and golden outputs
```

## Roadmap

- [ ] Define v0.1 DSL grammar
- [ ] Parser and AST
- [ ] Validation and diagnostics
- [ ] Graph model and connectivity analysis
- [ ] SVG renderer
- [ ] CLI
- [ ] JSON analysis output
- [ ] Lint rules
- [ ] Open/closed state and scenarios
- [ ] Documentation site

## Contributing

Contributions are welcome. For now, the most useful contributions are:

- DSL design feedback
- Example SLDs from real projects
- Analysis rules
- Renderer layout ideas
- Test fixtures

Please open an issue before large changes.

## License

MIT