# Vendored symbol collections

Reference artwork for the BusBar renderer's symbol registries
(`busbar-render`, spec §16.2). Per the implementation plan §5.6/§11 the
symbols we *ship in the renderer* are drawn from scratch as vector
primitives (licensing + determinism). These vendored collections serve as:

1. **Visual reference** for drawing and reviewing our primitive symbols,
2. **Renderer fixtures** (decode/geometry tests, golden comparisons), and
3. A permissively-licensed fallback if we ever choose to embed artwork
   directly — both licenses below permit it.

| Directory | Source | License | Contents |
|---|---|---|---|
| `electrical-symbol-library/` | [basverdoes/ElectricalSymbolLibrary](https://github.com/basverdoes/ElectricalSymbolLibrary) @ `ed1c2a3` | **CC0 1.0** (symbols only; repo scaffolding is CC BY-NC-SA and was **not** vendored) | 74 SVGs: `analog-iec/` (core, semiconductors, transducers), `analog-ansi/` (core, semiconductors, transducers), `other/` |
| `circuit-symbol-svg/` | [generalistprogrammer/circuit-symbol-svg](https://github.com/generalistprogrammer/circuit-symbol-svg) @ `528617f` | **MIT** (see its `LICENSE`) | 10 SVGs: battery, capacitor, diode, fuse, ground, inductor, lamp, led, resistor, switch-spst |

`.source-sha` files record the vendored commit of each collection.

## Coverage vs the M5 IEC registry

The plan's M5 scope (24 symbols: breaker, disconnector, fuse, transformer
2w, CT, VT, relay, motor, generator, grid, PV, battery, inverter, UPS,
earth, NGR, bus/section, board, ATS, meter, capacitor, reactor, cable
marker, SPD) is **not** covered by any single openly-licensed collection
found (2026-09 survey):

- These two sets are circuit-diagram oriented; useful items here:
  transformer (2-winding), fuse, ground/earth, battery, lamp, capacitor,
  inductor/reactor, ammeter/voltmeter (CT/VT adjacent), sources, wire
  terminals.
- IEC 60617 *installation/SLD* symbol sets that exist (QElectroTech GPL,
  chille/electricalsymbols unlicensed, elec-mate app-walled, official IEC
  60617 DB paid) were excluded on licensing grounds.
- Missing symbols (breaker, disconnector, motor, generator, CT/VT, relay,
  busbar, …) are to be drawn as primitives per implementation plan §5.6,
  using these collections and the standards' conventions as reference.

Survey notes live in the commit that added this directory.
