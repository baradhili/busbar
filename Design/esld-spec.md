# ESLD — Electrical Single Line Diagram Language — Specification v0.1 (General Purpose)

**Status:** Draft for discussion
**Scope:** Text-based description of electrical single line diagrams at any scale — residential, commercial, industrial, and utility distribution — across AC and DC, LV through transmission voltages, with switchboards, bus sections, ties, transformers, protection and metering, multiple sources, operating states, and interlocks.
**Lineage:** Generalizes the RSLD v0.1 residential seed ([deepseek.md](deepseek.md)). ESLD is a strict superset in intent; RSLD's residential library and rules become a code profile plus type pack on top of ESLD. See Appendix A for the change list.

This document is implementation-neutral. It defines the language, its abstract semantics, and the interchange model. It does not prescribe a parser generator, runtime, renderer, or host language. A companion document, [esld-implementation.md](esld-implementation.md), plans a reference toolchain.

---

## 1. Purpose and scope

### 1.1 Goals

- Describe electrical **topology** — what is connected to what — as plain text, at any voltage level.
- Make **assemblies first-class**: switchboards, distribution boards, and bus **sections** with **ties** (main-tie-main, double-bus, split-bus arrangements).
- Make **multiple incomers and sources** first-class (utility feeds, generators, PV, storage) with priority, transfer, paralleling, and islanding semantics.
- Model **transformation** between voltage systems (MV/LV, HV/MV, AC/DC) explicitly.
- Model **protection and metering** — CTs, VTs, relays with ANSI/IEC device functions, trip targets, and protection zones.
- Describe **operating states** (normal, maintenance, emergency) and **interlocks** as data, checkable by tooling.
- Support **circuit-level** description (lighting, sockets, motor feeders with starters, capacitor banks, EVSE, HVAC).
- Be **diffable**, **reviewable**, and **version-controllable** in Git.
- Be **validatable**: catch design errors before anyone draws anything.
- Be **renderable**: layout and symbols are derived, not authored.
- Be **extensible**: a project can define its own device types and rule packs.

### 1.2 Non-goals

- Not a PCB/netlist format.
- Not a cable-routing, tray, or physical-layout format.
- Not a substitute for a licensed electrical design or jurisdiction-specific certification.
- Not a numeric power-flow or short-circuit simulation engine in v0.1 (reachability and constraint checking only; numerics are roadmap).
- Not a coordinate-based drawing format.
- Not a protection-relay settings format (settings are carried as opaque data, not computed).

### 1.3 Conformance keywords

MUST, MUST NOT, SHOULD, SHOULD NOT, MAY are used per RFC 2119.

---

## 2. Design principles

1. **Topology over geometry.** The document describes connectivity. Coordinates live only in optional layout hints.
2. **Assemblies are containers; buses are attachment points.** Devices and circuits belong to a board; circuits attach to a bus section; sections join via ties.
3. **Sources are explicit.** Every source is a node with declared capability. There is no implicit "the grid".
4. **Ports are typed.** A port declares direction, domain (AC/DC), and kind (power/neutral/earth/signal).
5. **Implicit where obvious, explicit where not.** A circuit inside a board attaches to that board's (sole) bus section by default. Multi-section boards and cross-board feeds are always explicit.
6. **Ratings are data, not prose.** Ampacity, kVA, kA, mA trip thresholds, impedance percent are typed quantities.
7. **Operating reality is modelled.** Devices have positions (open/closed), sources have status, and both are exercisable through states and scenarios.
8. **Validation is parameterized by code profile.** Rules that vary by jurisdiction or plant standard are declared in a `code` block.
9. **Forward compatible.** Unknown properties MUST be preserved by round-tripping implementations.

---

## 3. Conformance and versioning

### 3.1 Document header

Every document SHOULD begin with a profile declaration:

```
profile "esld/1.0";
```

Implementations MUST reject documents whose profile major version they do not support, unless a `--lenient` mode is explicitly requested.

### 3.2 File extension and media type

- Extension: `.esld`
- Media type: `application/vnd.esld+text` (provisional)

### 3.3 Conformance classes

| Class | Requirement |
|---|---|
| **Reader** | Parses the grammar, builds the IR, preserves unknown properties |
| **Validator** | Reader + executes the rule catalog in §15 |
| **Renderer** | Validator + produces SVG/PDF per §17 |
| **Solver** | Validator + evaluates states, interlocks, and scenarios per §13/§14 |

---

## 4. Lexical structure

### 4.1 Character encoding

UTF-8. Line endings LF or CRLF. A leading BOM MUST be tolerated and stripped.

### 4.2 Whitespace

Space, tab, CR, LF. Insignificant except as a token separator.

### 4.3 Comments

```
// line comment
/* block comment
   spanning lines */
```

Comments MUST be preserved in round-trip mode as attached trivia.

### 4.4 Identifiers

```ebnf
IDENT  ::= ( LETTER | "_" ) ( LETTER | DIGIT | "_" | "-" )* ;
```

Case-sensitive. Convention: `SCREAMING_SNAKE` for instance tags, `lower_snake` for type names and property keys.

### 4.5 Strings

```ebnf
STRING ::= '"' ( CHAR | escape )* '"' ;
escape ::= "\" ( '"' | "\" | "n" | "t" | "u" HEX{4} ) ;
```

### 4.6 Numbers, units, quantities

```ebnf
NUMBER   ::= DIGIT+ ( "." DIGIT+ )? EXPONENT? ;
EXPONENT ::= ( "e" | "E" ) ( "+" | "-" )? DIGIT+ ;
UNIT     ::= LETTER ( LETTER | DIGIT | "/" | "^" | "%" )* ;
QUANTITY ::= NUMBER ( WS? UNIT )? ;
```

A quantity is a number with an optional unit token immediately following, optionally separated by whitespace: `230V`, `230 V`, `2.5mm2`, `30mA`, `13.8kV`, `5kWh`, `250MVA`.

Implementations MUST normalize `mm^2` and `mm²` to the canonical `mm2`. Unit prefixes (k, M, m, µ/u) are part of the unit token and MUST be normalized into a base value internally.

### 4.7 Booleans, enums

```ebnf
BOOLEAN ::= "yes" | "no" | "true" | "false" ;
ENUM    ::= IDENT ;
```

Common position enums used by states and interlocks: `open`, `closed`, `in1`, `in2`, `both`, `auto`, `manual`, `on`, `off`. These are ordinary identifiers, not reserved words.

### 4.8 Reserved words

```
profile  include  note  voltsys  code  type  board  circuit  group
connect  scenario  state  interlock  zone  layout  set  expect  use
requires excludes  in  out  bidi  ac  dc  power  neutral  earth
signal  mechanical  yes  no  true  false  bus  protection  controller
require extends  ports  params
```

Reserved words MUST NOT be used as instance tags or type names without quoting.

The member-name component of a dotted reference MAY be a port or section name that collides with a reserved word (e.g. `GRID.out`, `MSB_A.in`, `MSB_A.bus`), because member position after `.` is unambiguous.

---

## 5. Unit vocabulary

| Dimension | Accepted units |
|---|---|
| Voltage | `V`, `kV`, `mV` |
| Current | `A`, `kA`, `mA` |
| Power (real) | `W`, `kW`, `MW` |
| Power (apparent) | `VA`, `kVA`, `MVA` |
| Power (reactive) | `var`, `kvar`, `Mvar` |
| Energy | `Wh`, `kWh`, `MWh` |
| Frequency | `Hz` |
| Impedance / resistance | `ohm`, `mohm`, `kohm`, `%` |
| Conductor size | `mm2`, `kcmil`, `AWG` |
| Length | `mm`, `m`, `km`, `ft`, `in` |
| Time | `ms`, `s`, `min`, `h` |
| Temperature | `C`, `F`, `K` |
| Angle | `deg` |
| Illuminance | `lx` |

Dimensionless ratios use `%` or a bare number (e.g. `impedance_pct = 6%` or `impedance_pct = 6`).

---

## 6. Grammar

```ebnf
(* ==========================================================
   ESLD Grammar v0.1
   ========================================================== *)

document        ::= header? statement* EOF ;

header          ::= "profile" STRING ";" ;

statement       ::= include_stmt
                  | note_stmt
                  | voltsys_decl
                  | code_decl
                  | type_decl
                  | node_decl
                  | board_decl
                  | group_decl
                  | connect_stmt
                  | scenario_decl
                  | state_decl
                  | interlock_decl
                  | zone_decl
                  | layout_decl ;

include_stmt    ::= "include" STRING ";" ;
note_stmt       ::= "note" STRING ";" ;

(* ---------- Values ---------- *)
value           ::= quantity | NUMBER | BOOLEAN | STRING | ENUM
                  | list | range | block | ref ;

quantity        ::= QUANTITY ;
list            ::= "[" ( value ( "," value )* )? "]" ;
range           ::= value ".." value ;
block           ::= "{" ( property | note_stmt )* "}" ;
property        ::= IDENT "=" value ";" ;
ref             ::= IDENT ( "." IDENT )* ;

(* ---------- Voltage systems ---------- *)
voltsys_decl    ::= "voltsys" IDENT "=" voltsys_body ";" ;
voltsys_body    ::= block | voltsys_positional ;
voltsys_positional ::= quantity "," phase_spec "," quantity ;
phase_spec      ::= "1ph" | "split" | "3ph" | "dc" | NUMBER "ph" ;

(* ---------- Jurisdiction / plant standard profile ---------- *)
code_decl       ::= "code" STRING block ;

(* ---------- Type declarations (device library) ---------- *)
type_decl       ::= "type" IDENT ( ":" type_ref )? "{" type_item* "}" ;
type_ref        ::= IDENT ( "." IDENT )* ;
type_item       ::= "extends" type_ref ";"
                  | "ports" "{" port_decl* "}"
                  | "params" "{" param_decl* "}"
                  | "requires" "{" requirement* "}"
                  | "excludes" "{" requirement* "}"
                  | property
                  | note_stmt ;

port_decl       ::= IDENT ":" port_class ( "(" arg ( "," arg )* ")" )? ";" ;
port_class      ::= "ac_in" | "ac_out" | "ac_bidi"
                  | "dc_in" | "dc_out" | "dc_bidi"
                  | "neutral" | "earth" | "signal" | "mechanical" ;
arg             ::= IDENT | value ;

param_decl      ::= IDENT ":" param_type ( "=" value )? "required"? ";" ;
param_type      ::= IDENT ( "(" value ( "," value )* ")" )? ;

requirement     ::= IDENT ( "(" value ( "," value )* ")" )? ";" ;

(* ---------- Nodes ---------- *)
node_decl       ::= IDENT ":" type_ref node_block? ";"? ;
node_block      ::= "{" ( property | note_stmt )* "}" ;

(* ---------- Assemblies ---------- *)
board_decl      ::= "board" IDENT ( ":" type_ref )? "{" board_item* "}" ;
board_item      ::= property
                  | node_decl
                  | bus_decl
                  | circuit_decl
                  | group_decl
                  | connect_stmt
                  | note_stmt ;

(* A bus section. Boards with no explicit `bus` have one implicit
   section named `bus`. *)
bus_decl        ::= "bus" IDENT ( ":" type_ref )? "{" ( property | note_stmt )* "}" ;

(* ---------- Circuits ---------- *)
typed_shorthand ::= type_ref ( "{" ( property | note_stmt )* "}" )? ";" ;

circuit_decl    ::= "circuit" IDENT ( ":" type_ref )? "{" circuit_item* "}" ;
circuit_item    ::= property
                  | protection_item
                  | controller_item
                  | node_decl
                  | group_decl
                  | connect_stmt
                  | note_stmt ;
protection_item ::= "protection" ":" typed_shorthand ;
controller_item ::= "controller" ":" typed_shorthand ;

(* ---------- Groups ---------- *)
group_decl      ::= "group" IDENT "=" list ";" ;

(* ---------- Connections ---------- *)
connect_stmt    ::= endpoint ( arrow endpoint )+ connect_attrs? ";" ;
endpoint        ::= ref ( "[" phase_list "]" )? ;
phase_list      ::= IDENT ( "," IDENT )* ;
arrow           ::= "->" | "<-" | "--" ;
connect_attrs   ::= "{" ( property | note_stmt )* "}" ;

(* ---------- Scenarios ---------- *)
scenario_decl   ::= "scenario" STRING "{" scenario_item* "}" ;
scenario_item   ::= "set" ref "." IDENT "=" value ";"
                  | "expect" expectation ";"
                  | "use" STRING ";"
                  | note_stmt ;
expectation     ::= "energized"    "(" ref ")"
                  | "de_energized" "(" ref ")"
                  | "islanded"     "(" ref ")"
                  | "power_at"     "(" ref ")" ( "<=" | ">=" ) quantity
                  | "violation"    "(" STRING ")" ( "==" | "!=" ) BOOLEAN ;

(* ---------- Operating states ---------- *)
state_decl      ::= "state" STRING "{" state_item* "}" ;
state_item      ::= "use" STRING ";"                 // inherit another state
                  | ref "=" IDENT ";"                // device position
                  | note_stmt ;

(* ---------- Interlocks ---------- *)
interlock_decl  ::= "interlock" STRING "{" interlock_item* "}" ;
interlock_item  ::= "require" boolean_expr ";"
                  | note_stmt ;

boolean_expr    ::= boolean_and ( "||" boolean_and )* ;
boolean_and     ::= boolean_unary ( "&&" boolean_unary )* ;
boolean_unary   ::= "!" boolean_atom | boolean_atom ;
boolean_atom    ::= "(" boolean_expr ")"
                  | ref "=" IDENT                    // position test
                  | ref ;                            // sugar: ref is closed

(* ---------- Protection zones (advisory) ---------- *)
zone_decl       ::= "zone" STRING "{" ( property | note_stmt )* "}" ;

(* ---------- Layout hints ---------- *)
layout_decl     ::= "layout" "{" layout_item* "}" ;
layout_item     ::= "rank"    "=" IDENT ";"
                  | "flow"    "=" IDENT ";"
                  | "spacing" "=" quantity ";"
                  | ref "{" ( IDENT "=" value ";" )* "}"
                  | note_stmt ;
```

### 6.1 Arrow semantics

| Arrow | Meaning |
|---|---|
| `->` | Nominal supply direction (source → load). Does **not** restrict power flow. |
| `<-` | Reverse of the above; sugar, MUST be normalized to `->`. |
| `--` | Undirected / peer connection. Used for bus ties, signal, earth, and bus-tie-like links. |

Power flow direction is computed by the solver from source capability, not from arrow direction.

Edge **role** is inferred when not given explicitly as a connect attribute:

| Situation | Inferred role |
|---|---|
| Endpoint is a source port, other side is not | `supply` |
| Either endpoint is a bus section / `bus` port | `feeder` if directed, `tie` if `--` between two bus sections |
| Endpoint kind is `earth` or `neutral` | `earth` / `neutral` |
| Either endpoint kind is `signal` | `signal` |
| Otherwise | `final` |

---

## 7. Semantic model (IR)

The document compiles to a directed multigraph plus metadata. The IR is the normative interchange format.

### 7.1 Entities

```
Document
  profile       : ProfileRef
  voltageSystems: [VoltageSystem]
  codeProfiles  : [CodeProfile]
  types         : [TypeDef]
  nodes         : [Node]
  boards        : [Board]        // also present in nodes (kind = container)
  groups        : [Group]
  edges         : [Edge]
  states        : [State]
  interlocks    : [Interlock]
  zones         : [Zone]
  scenarios     : [Scenario]
  layout        : LayoutHints?
```

```
Node
  tag          : string          // unique across document
  type         : TypeRef
  kind         : source | converter | storage | protective | switch
               | load | passive | container | measurement
  parent       : BoardRef?       // containment, not electrical
  voltageSystem: VoltageSystemRef?
  ports        : [Port]
  properties   : Map<string, Value>
  sourceSpan   : Span
```

```
Board : Node (kind = container)
  sections     : [NodeRef]       // bus sections, ordered; implicit single
                                 // section is tagged BOARD.bus
  incomers     : [PortRef]       // permitted external feeds into the board
  priority     : [NodeRef]?      // dispatch order for solver
  busbarRating : Quantity?       // applies when no explicit sections
  ways         : int?
  circuits     : [CircuitRef]
```

```
BusSection : Node (type = bus)
  rating       : Quantity?
  incomers     : [PortRef]       // permitted external feeds into this section
```

```
Circuit : Node (kind = circuit)
  board        : BoardRef
  section      : NodeRef?        // bus section; defaults to the sole section
  protection   : NodeRef         // breaker / mcb / rcbo / fuse
  controller   : NodeRef?        // contactor / starter / vfd
  phase        : PhaseRef?
  cable        : NodeRef | CableSpec?
  loads        : [NodeRef]
```

```
Edge
  from         : PortRef
  to           : PortRef
  role         : supply | feeder | final | earth | neutral | signal | tie | trip
  properties   : Map<string, Value>
  sourceSpan   : Span
```

```
Port
  name         : string
  direction    : in | out | bidi
  domain       : ac | dc | none
  kind         : power | neutral | earth | signal | mechanical
  phases       : [PhaseRef]?
  voltageSystem: VoltageSystemRef?  // port-level override (converter ports)
```

```
State
  name         : string
  inherits     : StateRef?
  positions    : Map<NodeRef, enum>
```

```
Interlock
  name         : string
  requirements : [BooleanExpr]
```

### 7.2 Reference form

- `TAG` — node reference. A board reference in an endpoint resolves to its sole section's `bus` port; a bus section reference resolves to the section itself.
- `TAG.PORT` — port reference.
- `BOARD.SECTION` — bus section reference.
- `BOARD.CIRCUIT` — circuit reference.
- `TAG.PORT[L2]` — phase-qualified port reference.

### 7.3 Identity and scoping

- Tags MUST be unique within a document. (Implementations MAY allow board-local scoping with `BOARD.CHILD` disambiguation; the canonical form is always fully qualified.)
- A `circuit`, `node`, or `bus` declared inside a board is implicitly a member of that board, unless `parent = ...` overrides it.
- `include` files share one tag namespace. Include cycles are an error (R-112). Includes resolve relative to the including file, then a tool-provided search path.

### 7.4 JSON serialization (normative shape)

```json
{
  "profile": "esld/1.0",
  "voltageSystems": [
    { "id": "MV", "nominal": 11000, "unit": "V", "phases": 3,
      "frequency": 50, "neutral": true, "earthing": "resistance",
      "faultMva": { "value": 250, "unit": "MVA" } },
    { "id": "LV", "nominal": 400, "unit": "V", "phases": 3,
      "frequency": 50, "neutral": true, "earthing": "TN-S" }
  ],
  "nodes": [
    { "tag": "GRID_A", "type": "grid", "kind": "source",
      "ports": [{ "name": "out", "direction": "out",
                  "domain": "ac", "kind": "power" }],
      "properties": { "vs": "MV", "supply_a": { "value": 400, "unit": "A" } } },
    { "tag": "MV_SWBD", "type": "board", "kind": "container",
      "sections": ["MV_SWBD.A", "MV_SWBD.B"],
      "properties": { "vs": "MV" } },
    { "tag": "MV_SWBD.A", "type": "bus", "kind": "passive",
      "properties": { "rating_a": { "value": 1250, "unit": "A" },
                      "incomers": ["CB_IA.out"] } }
  ],
  "edges": [
    { "from": "GRID_A.out", "to": "CB_IA.in", "role": "supply" },
    { "from": "MV_SWBD.A", "to": "MV_SWBD.TIE_MV.a", "role": "tie" },
    { "from": "MV_SWBD.TIE_MV.b", "to": "MV_SWBD.B", "role": "tie" }
  ],
  "states": [
    { "name": "normal",
      "positions": { "CB_IA": "closed", "CB_IB": "closed",
                     "MV_SWBD.TIE_MV": "open" } }
  ]
}
```

---

## 8. Built-in type library

The following types MUST be available without declaration. Implementations MAY add more. `control_relay` replaces RSLD's `relay`; `relay` is now the protection relay.

### 8.1 Sources

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `grid` | source | `out: ac_bidi` | `vs`, `supply_a`, `fault_mva`, `x_r`, `export_allowed` |
| `generator` | source | `out: ac_out` | `vs`, `kva`, `kw`, `duty` (`prime`/`standby`), `fuel`, `start_s`, `bonded_neutral`, `xpd_pct` |
| `pv_array` | source | `out: dc_out` | `kw_peak`, `voc`, `isc`, `strings`, `tilt`, `azimuth` |
| `pv_string` | source | `out: dc_out` | `panels`, `w_peak`, `voc`, `isc` |
| `wind_turbine` | source | `out: ac_out` | `kw`, `vs`, `cut_in_ms`, `cut_out_ms` |

### 8.2 Conversion

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `transformer` | converter | `primary: ac_in`, `secondary: ac_out`, `tertiary: ac_out?`, `n: neutral?` | `kva`, `vs_in`, `vs_out`, `vector`, `impedance_pct`, `taps_count`, `tap_step_pct`, `cooling` |
| `inverter` | converter | `dc_in`, `ac_in`, `ac_out`, `backup_out` | `kind` (`string`/`hybrid`/`battery`/`micro`), `kw`, `mppt_count`, `export_limit`, `island_capable`, `transfer_ms` |
| `rectifier` | converter | `ac_in`, `dc_out` | `kw`, `v_out`, `regulation`, `float_v` |
| `ups` | converter | `ac_in`, `bypass_in: ac_in`, `ac_out`, `batt: dc_bidi` | `kva`, `kw`, `transfer_ms`, `backup_min` |

### 8.3 Storage and DC

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `battery` | storage | `dc_bidi` | `kwh`, `kw_charge`, `kw_discharge`, `chemistry`, `soc_min`, `v_nominal` |
| `bms` | passive | `signal` | `cells`, `comms` |
| `dc_load` | load | `in: dc_in` | `kw`, `v_nominal` |

### 8.4 Switchgear and switching

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `breaker` | switch | `in`, `out`, `trip: signal?` | `rating_a`, `poles`, `breaking_ka`, `technology` (`miniature`/`mccb`/`acb`/`air`/`vcb`/`sf6`/`ocb`), `trip_unit` (`thermal_magnetic`/`electronic`/`lsi`) |
| `disconnector` | switch | `in`, `out` | `rating_a`, `poles`, `motorized`, `lockable` (no load-break) |
| `load_break_switch` | switch | `in`, `out` | `rating_a`, `making_ka`, `poles` |
| `earth_switch` | switch | `in: earth` | `rating_a`, `motorized`, `lockable` |
| `contactor` | switch | `in`, `out`, `coil: signal` | `rating_a`, `poles`, `coil_v`, `utilization` (`AC-1`/`AC-3`...) |
| `control_relay` | switch | `in`, `out`, `coil: signal` | `rating_a`, `coil_v` |
| `ats` | switch | `in1`, `in2`, `out` | `priority`, `transfer_s`, `break_before_make`, `closed_transition` |
| `changeover` | switch | `in1`, `in2`, `out` | `mode` (`manual`/`auto`) |
| `main_switch` | switch | `in`, `out` | `rating_a`, `poles`, `lockable` |
| `isolator` | switch | `in`, `out` | `rating_a`, `poles`, `lockable` |

### 8.5 Protection devices

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `mcb` | protective | `in`, `out` | `rating_a`, `curve`, `poles`, `breaking_ka` |
| `mccb` | protective | `in`, `out` | `rating_a`, `curve`, `poles`, `breaking_ka`, `adjustable` |
| `rcbo` | protective | `in`, `out` | `rating_a`, `curve`, `rcd_ma`, `rcd_type`, `poles`, `breaking_ka` |
| `rcd` | protective | `in`, `out` | `rcd_ma`, `rcd_type`, `poles`, `s_type` |
| `afci` | protective | `in`, `out` | `rating_a`, `poles` |
| `spd` | protective | `in`, `pe` | `spd_type` (1/2/3), `up_kv`, `in_ka` |
| `fuse` | protective | `in`, `out` | `rating_a`, `class`, `breaking_ka` |

### 8.6 Measurement and protection systems

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `ct` | measurement | `signal` | `primary_a`, `secondary_a`, `measures`, `class_p`, `burden_va` |
| `vt` | measurement | `signal` | `primary_v`, `secondary_v`, `kind` (`inductive`/`capacitive`), `measures` |
| `relay` | protective | `signal` | `functions` (list of strings, ANSI/IEC device numbers), `trips` (list of node refs), `ct`, `vt`, `settings` (opaque block) |
| `sync_check` | measurement | `signal` | `dead_bus_permitted`, `max_slip_hz`, `max_v_diff_pct` |
| `meter` | measurement | `in`, `out` | `kind` (`utility`/`sub`/`bidirectional`), `accuracy`, `ct`, `vt` |

`ct.measures` / `vt.measures` / `meter.ct` / `relay.ct` create **signal** edges (rendered dotted). `relay.trips` creates **trip** edges to breakers (rendered dashed).

### 8.7 Passive distribution and grounding

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `bus` | passive | (attachment point) | `rating_a`, `ip_rating` |
| `busbar` | passive | `*` | `rating_a`, `material`, `sections` |
| `junction` | passive | `a`, `b`, `c`, `d` | `rating_a` |
| `cable` | passive | `a`, `b` | `csa`, `cores`, `conductor` (`cu`/`al`), `insulation`, `length`, `method`, `ampacity_a` |
| `line` | passive | `a`, `b` | `conductor`, `length_km`, `ampacity_a` |
| `ngr` | passive | `a`, `b` | `ohm`, `current_10s_a`, `material` |
| `reactor` | passive | `a`, `b` | `kvar`, `ohm`, `q_factor` |
| `earth` | passive | `e: earth` | `electrode`, `ohm` |
| `capacitor_bank` | load | `in` | `kvar`, `stages`, `harmonic_tuned` |

### 8.8 Loads

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `load` | load | `in` | `kw`, `pf`, `duty`, `diversity`, `essential` |
| `lighting` | load | `in` | `kw`, `points`, `control` |
| `socket` | load | `in` | `kw`, `points`, `rcd_required` |
| `appliance` | load | `in` | `kw`, `dedicated` |
| `motor` | load | `in` | `kw`, `phases`, `pf`, `starter` (`none`/`dol`/`star_delta`/`soft`/`vfd`/`rheostat`), `duty`, `poles` |
| `heating` | load | `in` | `kw`, `phases` |
| `oven`, `cooktop` | load | `in` | `kw`, `phases` |
| `hws` | load | `in` | `kw`, `litres`, `controlled_load` |
| `hvac` | load | `in` | `kw`, `phases`, `cop`, `compressor` |
| `pool_pump` | load | `in` | `kw`, `phases` |
| `evse` | load | `in` | `kw`, `phases`, `mode`, `v2x_capable` |

### 8.9 Containers

| Type | Kind | Ports | Key parameters |
|---|---|---|---|
| `board` | container | `in`, `out`, `n`, `pe`, `bus` | `busbar_rating_a`, `ways`, `location`, `ip_rating`, `form` (`fixed`/`drawout`) |

### 8.10 Example type declaration

```
type vcb_breaker : breaker {
  extends breaker;
  ports {
    in   : ac_in;
    out  : ac_out;
    trip : signal;
  }
  params {
    rating_a    : quantity(A) required;
    poles       : int = 3;
    breaking_ka : quantity(kA) required;
    technology  : enum(vcb, sf6, air, ocb) = vcb;
  }
  requires {
    breaking_ka >= 25kA;
  }
};
```

---

## 9. Assemblies: boards, bus sections, ties, circuits

### 9.1 Board semantics

A `board` is a container node that owns:

- zero or more **bus sections** (`bus` declarations). A board with no explicit section has exactly one implicit section, tagged `BOARD.bus`.
- zero or more `circuit` children, each attached to a section
- zero or more protection, switching, metering, or conversion devices
- an `incomers` list: the ports permitted to feed the board or its sections from outside

A board MUST declare or inherit:

- a voltage system (`vs`)
- a busbar rating (`busbar_rating_a`, or per-section `rating_a`) if any backfeed source is present

### 9.2 Bus sections and incomers

```
board MV_SWBD : board {
  vs = MV;

  bus A { incomers = [CB_IA.out]; rating_a = 1250A; }
  bus B { incomers = [CB_IB.out]; rating_a = 1250A; }

  tie TIE_MV : breaker { rating_a = 630A; breaking_ka = 25kA; technology = vcb; }
  connect A -- TIE_MV -- B;
}
```

Rules:

- External feeds MUST terminate on a section reference (`MV_SWBD.A`) or the board's `bus` port, and MUST appear in that section's (or the board's, for single-section boards) `incomers` list — else **R-110**.
- Internal circuits attach to a section via the `bus` property and are not gated by `incomers`.
- A protective, switching, or measurement device declared inside a board without explicit connections attaches implicitly to the sole section's busbar at its `in` port. In a multi-section board, implicit attach of devices is an error (R-113), same as for circuits.
- A circuit in a multi-section board without `bus = ...` is **R-113** (error). In a single-section board it attaches to the implicit section.
- Feeding `BOARD.in` is equivalent to feeding `BOARD.bus` and exists for RSLD compatibility; multi-section boards SHOULD use explicit section references.

### 9.3 Circuit semantics

A circuit is a composite node:

```
circuit MCC_WATER {
  bus         = A;
  protection  : mccb { rating_a = 160A; breaking_ka = 35kA; };
  controller  : contactor { rating_a = 160A; utilization = "AC-3"; };
  cable       = { csa = 50mm2; cores = 4; method = "cable_ladder"; };
  phase       = L1;
  loads       = [PUMP_WATER];
}
```

Expansion:

1. The `protection` device becomes a node in the board.
2. Its `in` port attaches to the selected bus section.
3. If `controller` is present it is inserted in series after protection (breaker → contactor → cable).
4. Its `out` port (protection or controller) is the circuit's supply point, addressable as `BOARD.CIRCUIT.out`.
5. The `cable` becomes a `cable` node between the supply point and loads.
6. Every load in `loads` attaches to the circuit supply point.

A circuit MAY declare `fed_by = <ref>` to bypass implicit busbar attachment (used for switched or sub-fed circuits).

### 9.4 Ties

A tie is a switching device connected between two bus sections with `--`:

```
connect MV_SWBD.A -- TIE_MV -- MV_SWBD.B;
```

Ties between sections of different boards are permitted (`connect MSB_A.bus -- CB_TIE_LV -- MSB_B.bus;`). The tie device carries its own rating and protection as a normal node. Normally-open ties are expressed in `state` blocks, not in the topology.

### 9.5 Sub-boards

A sub-board is a `board` fed from a parent board's circuit or bus section:

```
board GARAGE : board {
  vs = LV;
  busbar_rating_a = 63A;
  ways = 12;
}

circuit GARAGE_FEED {
  protection : rcbo { rating_a = 40A; rcd_ma = 30mA; };
  cable      = { csa = 6mm2; cores = 3; length = 18m; };
}

connect MAIN.GARAGE_FEED.out -> GARAGE.in;
```

The sub-board's incomers list (if declared) MUST include `MAIN.GARAGE_FEED.out`. Nesting depth is unbounded; implementations SHOULD warn beyond 3 levels for LV boards.

---

## 10. Sources, paralleling, transfer, islanding

### 10.1 Incomers

`incomers` (board- or section-level) declares the set of permitted feeds. The solver MUST reject any external connection to a board/section from a source not in `incomers`.

### 10.2 Priority

`priority` gives the dispatch order used by the solver and by scenario evaluation:

```
board MAIN : board {
  incomers = [GRID.out, INV1.ac_out];
  priority = [INV1, BATT1, GRID];
}
```

Earlier entries are dispatched first. `priority` is advisory metadata for validation and scenarios; it does not change physical topology.

### 10.3 Transfer devices

Where a physical changeover exists, model it with `ats` or `changeover`:

```
ats ATS1 : ats {
  priority          = [GRID_A, GEN1];
  transfer_s        = 60ms;
  break_before_make = yes;
}

connect GRID_A.out -> ATS1.in1;
connect GEN1.out   -> ATS1.in2;
connect ATS1.out   -> ESSENTIALS.in;
```

An ATS input is *available* when any source listed in its `priority` is reachable through that input. The ATS energizes `out` from the first available input in priority order. Devices with `closed_transition = yes` may briefly parallel their inputs; this is subject to R-410.

### 10.4 Source status and standby generators

Sources carry a logical status in scenarios: `online` (default), `offline`, or `standby`.

- `offline`: source delivers nothing.
- `online`: source delivers per its capability.
- `standby`: the source is available only when a transfer device selects it and no higher-priority input is available. `generator` nodes with `duty = standby` default to `standby`; all other sources default to `online`.

### 10.5 Paralleling and synchronism

Two independent sources (different `grid` nodes, or a grid and a generator) energizing the same merged bus group is **paralleling**. It is permitted only when one of:

- a `sync_check` device is declared on the closing path, or
- an `interlock` explicitly permits the condition, or
- the sources descend from the same upstream source node.

Otherwise **R-410** applies (error). Transformers in parallel on the same pair of buses additionally SHOULD match `vector` and reasonably match `impedance_pct` (R-208, warning).

### 10.6 Islanding and essential loads

A converter with `island_capable = yes` (or a standby generator) can sustain a separated domain. Any board fed from an island-capable output MUST be reachable from that capability, and MUST NOT backfeed the grid side unless `export_allowed = yes`. A validator MUST flag circuits marked `essential = yes` that sit on a board not reachable from any island-capable source (R-405).

### 10.7 Energy flow model

For each scenario the solver computes:

1. The set of energized nodes, given device positions (from the active state) and source status.
2. Merged bus groups (sections joined by closed ties and closed paths).
3. For each merged group, the active incomer (first in `priority` that is available).
4. Net power at declared measurement points (sign only, plus limit checks against declared limits).
5. Constraint violations (export limit, backfeed rule, islanding breach, interlock breach).

The solver does not perform power-flow numerics in v0.1. `power_at(ref)` is sign-resolved (positive in the `->` arrow direction; export at a source is negative `power_at`) and bounded only where limits are declared. Numeric load flow is roadmap (§21).

---

## 11. Voltage systems, phasing, earthing

### 11.1 Declarations

```
voltsys MV = {
  nominal   = 11kV;
  phases    = 3ph;
  frequency = 50Hz;
  neutral   = yes;
  earthing  = resistance;     // solid | resistance | reactance | isolated | none
  fault_mva = 250MVA;         // prospective fault level at this bus
  x_r       = 8;
};

voltsys LV_3P = {
  nominal   = 400V;
  phases    = 3ph;
  frequency = 50Hz;
  neutral   = yes;
  earthing  = "TN-S";         // TN-S | TN-C | TN-C-S | TT | IT (LV systems)
};

voltsys LV_AU = { nominal = 230V; phases = 1ph; frequency = 50Hz; neutral = yes; earthing = "TN-C-S"; };
voltsys DC_AUX = { nominal = 48V; phases = dc; neutral = no; earthing = isolated; };
```

The positional form `voltsys LV = 230V, 1ph, 50Hz;` remains valid.

### 11.2 Phase naming

| System | Phase identifiers |
|---|---|
| `dc` | — (domain check only) |
| `1ph` | `L1` (alias `L`) |
| `split` | `L1`, `L2` |
| `3ph` | `L1`, `L2`, `L3` |
| `Nph` | `L1`..`LN` |

Neutral is `N`; protective earth is `PE`.

### 11.3 Phase assignment

- In `1ph` systems `phase` MAY be omitted.
- In `split` and `3ph` systems every circuit SHOULD declare `phase` (or `phases` for multi-phase loads).
- 3-phase feeders and motors declare `phases = [L1, L2, L3]` or omit `phase` entirely, which denotes all phases.
- Validators MUST report load imbalance in `split`/`3ph` boards exceeding a code-configured threshold.

### 11.4 Voltage compatibility

Two connected ports are compatible if:

- same `domain` (ac/dc), and
- same voltage system, **or** the edge terminates on a port of a converter node whose other side declares the other system (`transformer`, `inverter`, `rectifier`, `ups`), and
- same nominal frequency for AC.

Ports on converter nodes may carry a port-level `vs` argument: `secondary : ac_out(vs = LV_3P);`

### 11.5 Earthing compatibility

- LV systems with different `earthing` schemes (e.g. TN-S and TT) MUST NOT be paralleled or tied (**R-207**).
- Each islanded AC system MUST have at most one neutral-earth bond (**R-402**). Bonds are expressed by connecting a source/transformer `n` port to `earth` or `pe`, or by `bonded_neutral = yes` on a generator.
- MV/HV neutrals earthed through `ngr` nodes SHOULD have the NGR in the path: `connect TX1.n -> NGR1.a; connect NGR1.b -> PLANT_EARTH.e;`

---

## 12. Protection and metering model

### 12.1 CTs, VTs, meters

```
ct CT_IA : ct   { primary_a = 400A; secondary_a = 5A; measures = GRID_A; class_p = "5P20"; }
vt VT_A  : vt   { primary_v = 11000V; secondary_v = 110V; kind = inductive; measures = MV_SWBD.A; }
meter M1 : meter { kind = sub; accuracy = "0.5S"; ct = CT_IA; vt = VT_A; }
```

`measures`, `ct`, and `vt` properties create signal edges; they do not affect energization.

### 12.2 Relays

```
relay PROT_IA : relay {
  functions = ["50", "51", "50N", "59"];
  trips     = [CB_IA];
  ct        = CT_IA;
  vt        = VT_A;
  settings  = { pickup_a = 360A; td = 0.05; };
}
```

- `functions` is a list of strings: ANSI/IEEE device numbers (`"50"`, `"51"`, `"87"`, `"27"`, `"59"`, `"81"`, ...) or IEC prefixes (`"87T"`).
- `trips` lists the switching devices the relay trips — creates trip edges. Each target MUST be a `switch`-kind node (**R-313**).
- A relay with current-based functions (`50`, `51`, `50N`, `67`, `87`...) MUST reference a `ct` (**R-312**).
- `settings` is opaque preserved data. ESLD carries settings; it does not compute or verify curves in v0.1 (roadmap).

### 12.3 Zones

```
zone "T1 differential" {
  members = [CB_MV_T1, T1.primary, T1.secondary, CB_LV_T1];
  note "87T zone; CTs at both breakers.";
}
```

Zones are advisory in v0.1: renderers SHOULD shade zone members; validators check that `members` form a connected subgraph (warning if not). Zones do not affect energization.

---

## 13. Operating states and interlocks

### 13.1 States

```
state "normal" {
  CB_IA  = closed;
  CB_IB  = closed;
  TIE_MV = open;
  TIE_LV = open;
  ATS1   = in1;
}

state "MV incomer A maintenance" {
  use "normal";          // inherit, then override
  CB_IA  = open;
  TIE_MV = closed;
}
```

- Position assignment targets MUST reference `switch`-kind nodes (**R-114** otherwise).
- A state inherits positions from its `use` target, then overrides.
- Positions are logical enums; common values `open`/`closed` (switching devices), `in1`/`in2`/`both` (transfer devices), `auto`/`manual`.
- Unspecified devices default to their type default (breakers, switches: `closed`; transfer devices: `in1`; disconnector: `closed`).

### 13.2 Interlocks

```
interlock "no parallel of independent sources" {
  require !(TIE_MV = closed) || !(CB_IA = closed && CB_IB = closed);
}

interlock "ATS never parallels inputs" {
  require !(ATS1 = both);
}
```

- A bare reference `TIE_MV` means `TIE_MV = closed`.
- Every declared interlock MUST evaluate true under every declared state that, combined with default positions, is reachable — else **R-603** (error).
- Interlocks participate in scenario evaluation: a scenario whose resulting positions violate an interlock fails (**R-603**).

### 13.3 Earth switches

An `earth_switch` in `closed` position while its bus group is energized is **R-602** (error) — checked in every state and scenario.

---

## 14. Scenarios

Scenarios exercise the design without changing the base topology.

```
scenario "Loss of grid incomer A" {
  use "normal";
  set GRID_A.status = offline;

  expect de_energized(MSB_A);
  expect energized(ESSENTIALS);
  expect islanded(GEN1);
  expect power_at(GRID_A.out) >= 0kW;
}

scenario "Peak PV export" {
  set GRID_A.status  = online;
  set PV1.irradiance = 1000W/m2;
  set BATT1.soc      = 100%;

  expect power_at(GRID_A.out) <= -5MW;
  expect violation("R-401") == false;
}
```

Semantics:

1. `use "state"` applies device positions (states may chain via their own `use`).
2. `set ref.prop = value` overrides a node property for the scenario. Recognized settables include `status` on sources, `soc` on storage, `irradiance` on PV. Unrecognized `set` targets are an error.
3. `expect` clauses are evaluated against the solved result; unsatisfied expectations are **R-501**.
4. `power_at` sign: positive in the `->` arrow direction. Export at a grid port (arrow pointing into the installation) is negative.

---

## 15. Validation rule catalog

Rule IDs are stable. Implementations MUST report the ID and source span. Code-dependent rules read parameters from the active `code` profile(s).

### R-100 — Structural

| ID | Rule | Severity |
|---|---|---|
| R-101 | Unknown type reference | error |
| R-102 | Duplicate tag in document | error |
| R-103 | Required port unconnected | error |
| R-104 | Port arity violated (e.g. two feeds into a single `in`) | error |
| R-105 | Node unreachable from any source | warning |
| R-106 | Board or bus section with no incomer and no internal feed | error |
| R-107 | Board with no outgoing circuits | warning |
| R-108 | Circular supply path with no source | error |
| R-109 | Orphan sub-board (declared, never fed) | error |
| R-110 | External feed not listed in `incomers` | error |
| R-111 | Connection to a non-existent port | error |
| R-112 | Include cycle or missing include | error |
| R-113 | Circuit or board-declared device without `bus` on a multi-section board | error |
| R-114 | State/interlock position on a non-switch node | error |

### R-200 — Voltage, phase, frequency, earthing

| ID | Rule | Severity |
|---|---|---|
| R-201 | Voltage system mismatch across an edge (no converter boundary) | error |
| R-202 | Frequency mismatch across an edge | error |
| R-203 | Phase missing on a multi-phase board | error |
| R-204 | Phase imbalance exceeds code threshold | warning |
| R-205 | Multi-phase load on a single-phase system | error |
| R-206 | Neutral connected where no neutral exists | error |
| R-207 | Parallel or tie between different earthing schemes | error |
| R-208 | Paralleled transformers with incompatible vector group or far-off impedance | warning |

### R-300 — Protection and rating

| ID | Rule | Severity |
|---|---|---|
| R-301 | Protective device rating exceeds cable ampacity | error |
| R-302 | Connected load exceeds protective device rating | error |
| R-303 | Sum of circuit ratings exceeds board main rating | warning |
| R-304 | Socket circuit without RCD (code-dependent) | error |
| R-305 | SPD absent at board incomer (code-dependent) | warning |
| R-306 | Breaking capacity below prospective fault current | error |
| R-307 | No main switch or isolator between board incomer and bus | error |
| R-308 | RCD selectivity violated between upstream/downstream | warning |
| R-309 | AFCI required but absent (code-dependent) | error |
| R-310 | Switchgear continuous rating below circuit load | error |
| R-311 | Transformer kVA below connected load after diversity | warning |
| R-312 | Current-function relay without CT | error |
| R-313 | Relay `trips` target is not a switching device | error |

### R-400 — Sources, paralleling, islands

| ID | Rule | Severity |
|---|---|---|
| R-401 | Backfeed source sum exceeds code % of busbar rating | error |
| R-402 | More than one neutral-earth bond per islanded system | error |
| R-403 | Grid-tie inverter without anti-islanding declared | error |
| R-404 | Battery discharge path to grid without `export_allowed` | error |
| R-405 | Circuit marked `essential` not reachable from island-capable source | error |
| R-406 | Aggregate export limit exceeded by declared sources | warning |
| R-407 | Generator neutral bonded while grid neutral also bonded | error |
| R-408 | Transfer device without declared `priority` | warning |
| R-409 | Backup board fed from a non-island-capable source | error |
| R-410 | Paralleling independent sources without sync-check or permitting interlock | error |

### R-500 — Scenarios

| ID | Rule | Severity |
|---|---|---|
| R-501 | Scenario expectation not satisfied | error |
| R-502 | Scenario leaves an essential circuit de-energized | error |
| R-503 | Scenario power flow above a declared limit | warning |

### R-600 — States and interlocks

| ID | Rule | Severity |
|---|---|---|
| R-601 | A state energizes a merged bus group from independent unsynchronized sources | error |
| R-602 | Earth switch closed while its bus group is energized | error |
| R-603 | A declared interlock is violated by a state or scenario result | error |
| R-604 | A state de-energizes a circuit marked `essential` | error |

### 15.1 Code profile example

```
code "AS/NZS 3000:2018" {
  rcd_socket_circuits     = 30mA;
  rcd_lighting_circuits   = none;
  max_final_circuit_a     = 32A;
  spd_required            = yes;
  neutral_earth_bond      = "MEN at main switchboard only";
  pv_backfeed_limit_pct   = 100;
  phase_imbalance_max_pct = 20;
}

code "NEC 2023" {
  rcd_socket_circuits   = 5mA;    // GFCI
  afci_required         = yes;
  pv_backfeed_limit_pct = 120;
  spd_required          = yes;
}

code "plant-standard/2026" {
  max_final_circuit_a    = 63A;
  parallel_interlock_required = yes;
  fault_margin_pct       = 20;    // breaking_ka must exceed fault by this margin
}
```

Multiple `code` blocks MAY be active; later declarations override earlier keys. `none` disables a code-dependent rule.

---

## 16. Layout hints and rendering contract

### 16.1 Layout block

```
layout {
  rank      = source_to_load;
  flow      = top_to_bottom;
  spacing   = 40mm;

  MV_SWBD {
    orientation = horizontal;
    busbar      = top;
    group_by    = section;
  }

  ESSENTIALS { column = 2; }

  redundancy_group A_FEEDS { color = accent1; }
}
```

Layout hints are **advisory**. A renderer that cannot satisfy them MUST still produce a valid drawing.

### 16.2 Renderer requirements

A conforming renderer:

- MUST place boards as labelled rectangles with a busbar representation; multi-section boards show one bar per section, visually joined through tie devices.
- MUST draw protective devices between busbar and circuit load.
- MUST draw sources with distinguishable symbols by type.
- MUST label every node with its tag.
- MUST annotate protective devices with rating and curve; breakers with rating and breaking capacity.
- MUST render ties between bus sections as switching devices spanning the bars.
- SHOULD render measurement/trip links (CT, VT, relay, meter) as thin dotted/dashed lines distinct from power conductors.
- SHOULD support both IEC 60617 and ANSI/IEEE 315 symbol sets, selected by a render option.
- SHOULD shade declared protection zones.
- SHOULD route feeders orthogonally.
- MUST NOT require coordinates in the source document.
- MUST produce deterministic output for identical input (stable layout).

### 16.3 Minimum readable output

For a 30-circuit installation with two boards, a bus tie, and three sources, an A3 landscape SVG SHOULD be legible at 100% zoom without manual adjustment.

---

## 17. Interchange and export

| Format | Purpose | Requirement |
|---|---|---|
| ESLD | Canonical source | MUST round-trip |
| JSON IR | Machine interchange | MUST be derivable; SHOULD be loadable |
| DOT | Debug / graph inspection | SHOULD |
| CSV | Board schedule, circuit schedule | SHOULD |
| SVG | Drawing | Renderer conformance |
| PDF | Drawing | Renderer conformance |
| YAML | Alternative source syntax | MAY |

### 17.1 Round-trip invariant

```
parse(serialize(parse(text))) == parse(text)
```

Unknown properties, comments, and ordering MUST be preserved in round-trip mode.

### 17.2 Schedule export

A CSV export of circuits MUST include, at minimum:

```
board, section, circuit, protection_type, rating_a, curve, breaking_ka, rcd_ma,
poles, phase, controller_type, cable_csa, cable_cores, load_summary_kw, essential
```

---

## 18. Worked examples

### 18.1 Industrial plant: dual MV incomers, main-tie-main, standby generator

```
profile "esld/1.0";

voltsys MV = { nominal = 11kV; phases = 3ph; frequency = 50Hz;
               neutral = yes; earthing = resistance; fault_mva = 250MVA; };
voltsys LV = { nominal = 400V; phases = 3ph; frequency = 50Hz;
               neutral = yes; earthing = "TN-S"; };

code "plant-standard/2026" {
  max_final_circuit_a      = 63A;
  parallel_interlock_required = yes;
  fault_margin_pct         = 20;
};

grid GRID_A : grid { vs = MV; supply_a = 400A; fault_mva = 250MVA; x_r = 8; export_allowed = no; };
grid GRID_B : grid { vs = MV; supply_a = 400A; fault_mva = 250MVA; x_r = 8; export_allowed = no; };

// ---------- MV switchboard: main-tie-main ----------
board MV_SWBD : board {
  vs = MV;

  ct CT_IA : ct { primary_a = 400A; secondary_a = 5A; measures = GRID_A; class_p = "5P20"; }
  relay PROT_IA : relay { functions = ["50", "51", "50N"]; trips = [CB_IA]; ct = CT_IA; }
  breaker CB_IA : breaker { rating_a = 630A; breaking_ka = 31.5kA; technology = vcb; }

  ct CT_IB : ct { primary_a = 400A; secondary_a = 5A; measures = GRID_B; class_p = "5P20"; }
  relay PROT_IB : relay { functions = ["50", "51", "50N"]; trips = [CB_IB]; ct = CT_IB; }
  breaker CB_IB : breaker { rating_a = 630A; breaking_ka = 31.5kA; technology = vcb; }

  vt VT_A : vt { primary_v = 11000V; secondary_v = 110V; kind = inductive; measures = A; }

  bus A { incomers = [CB_IA.out]; rating_a = 1250A; }
  bus B { incomers = [CB_IB.out]; rating_a = 1250A; }

  tie TIE_MV : breaker { rating_a = 630A; breaking_ka = 31.5kA; technology = vcb; }
  connect A -- TIE_MV -- B;

  circuit TXA_FEED {
    bus        = A;
    protection : breaker { rating_a = 200A; breaking_ka = 31.5kA; technology = vcb; };
  }

  circuit TXB_FEED {
    bus        = B;
    protection : breaker { rating_a = 200A; breaking_ka = 31.5kA; technology = vcb; };
  }
}

connect GRID_A.out -> CB_IA.in;
connect GRID_B.out -> CB_IB.in;

// ---------- Transformation ----------
transformer TX_A : transformer {
  kva = 1500kVA; vs_in = MV; vs_out = LV; vector = "Dyn11";
  impedance_pct = 6; taps_count = 5; tap_step_pct = 2.5%; cooling = ONAN;
}
transformer TX_B : transformer {
  kva = 1500kVA; vs_in = MV; vs_out = LV; vector = "Dyn11";
  impedance_pct = 6; taps_count = 5; tap_step_pct = 2.5%; cooling = ONAN;
}

earth PLANT_EARTH : earth { electrode = "grid"; ohm = 1ohm; }
ngr NGR_MV : ngr { ohm = 12ohm; current_10s_a = 400A; }

connect MV_SWBD.TXA_FEED.out -> TX_A.primary;
connect MV_SWBD.TXB_FEED.out -> TX_B.primary;
connect TX_A.n -> NGR_MV.a;         // MV neutral earthed via NGR
connect NGR_MV.b -> PLANT_EARTH.e;
connect TX_B.n -> PLANT_EARTH.e;

// ---------- LV main boards: main-tie-main ----------
board MSB_A : board {
  vs = LV;
  incomers = [TX_A.secondary];

  main_switch MAIN_A : breaker { rating_a = 2500A; breaking_ka = 50kA; technology = acb; }
  spd SPD_A : spd { spd_type = 2; in_ka = 40kA; }
  meter M_A : meter { kind = sub; ct = CT_IA; }

  tie CB_TIE_LV : breaker { rating_a = 2000A; breaking_ka = 50kA; technology = acb; }
  connect MSB_A.bus -- CB_TIE_LV -- MSB_B.bus;

  circuit MCC_WATER {
    protection : mccb { rating_a = 160A; breaking_ka = 35kA; };
    controller : contactor { rating_a = 160A; utilization = "AC-3"; };
    cable      = { csa = 50mm2; cores = 4; method = "cable_ladder"; };
    loads      = [PUMP_WATER];
  }

  circuit CAP_BANK {
    protection : mccb { rating_a = 100A; breaking_ka = 35kA; };
    loads      = [CAP1];
  }

  circuit ESSENTIALS_FEED {
    protection : mccb { rating_a = 250A; breaking_ka = 35kA; };
    cable      = { csa = 120mm2; cores = 4; length = 45m; };
  }

  circuit LIGHTS_AREA_A {
    protection : rcbo { rating_a = 20A; curve = C; rcd_ma = 30mA; };
    cable      = { csa = 2.5mm2; cores = 5; method = "clipped" };
    loads      = [LIGHTS_A1, LIGHTS_A2];
  }
}

board MSB_B : board {
  vs = LV;
  incomers = [TX_B.secondary];

  main_switch MAIN_B : breaker { rating_a = 2500A; breaking_ka = 50kA; technology = acb; }
  spd SPD_B : spd { spd_type = 2; in_ka = 40kA; }

  circuit MCC_AIR {
    protection : mccb { rating_a = 125A; breaking_ka = 35kA; };
    controller : contactor { rating_a = 125A; utilization = "AC-3"; };
    loads      = [FAN_AIR_1, FAN_AIR_2];
  }

  circuit HVAC_PLANT {
    protection : mccb { rating_a = 250A; breaking_ka = 35kA; };
    cable      = { csa = 95mm2; cores = 4; length = 60m; };
    loads      = [CHILLER_1];
  }
}

connect TX_A.secondary -> MSB_A.MAIN_A.in;
connect MSB_A.MAIN_A.out -> MSB_A.bus;
connect TX_B.secondary -> MSB_B.MAIN_B.in;
connect MSB_B.MAIN_B.out -> MSB_B.bus;

motor PUMP_WATER : motor { kw = 75kW; pf = 0.85; starter = dol; };
motor FAN_AIR_1  : motor { kw = 45kW; pf = 0.85; starter = vfd; };
motor FAN_AIR_2  : motor { kw = 45kW; pf = 0.85; starter = vfd; };
hvac CHILLER_1   : hvac  { kw = 220kW; cop = 5.1; compressor = screw; };
capacitor_bank CAP1 : capacitor_bank { kvar = 100kvar; stages = 3; };
lighting LIGHTS_A1 : lighting { kw = 4kW; points = 24; control = "dalí" };
lighting LIGHTS_A2 : lighting { kw = 3kW; points = 18; };

// ---------- Standby generation and essential board ----------
generator GEN1 : generator {
  vs = LV; kva = 500kVA; kw = 400kW; duty = standby;
  fuel = diesel; start_s = 10s; bonded_neutral = no;
}

ats ATS1 : ats {
  priority          = [GRID_A, GEN1];
  transfer_s        = 60ms;
  break_before_make = yes;
}

connect MSB_A.ESSENTIALS_FEED.out -> ATS1.in1;
connect GEN1.out -> ATS1.in2;
connect ATS1.out -> ESSENTIALS.in;

board ESSENTIALS : board {
  vs = LV;
  incomers = [ATS1.out];
  busbar_rating_a = 630A;

  circuit ESS_LIGHTS {
    protection : rcbo { rating_a = 10A; curve = B; rcd_ma = 30mA; };
    essential  = yes;
    loads      = [LIGHT_E_ROUTE];
  }

  circuit ESS_SOCKETS {
    protection : rcbo { rating_a = 16A; curve = C; rcd_ma = 30mA; };
    essential  = yes;
    loads      = [SOCK_E_PANEL];
  }

  circuit FIRE_PUMP {
    protection : mccb { rating_a = 125A; breaking_ka = 35kA; };
    essential  = yes;
    loads      = [PUMP_FIRE];
  }
}

motor PUMP_FIRE : motor { kw = 55kW; starter = dol; duty = "S1"; };
lighting LIGHT_E_ROUTE : lighting { kw = 1.2kW; points = 12; };
socket SOCK_E_PANEL : socket { kw = 2kW; points = 6; };

// ---------- Protection zone ----------
zone "T1 differential" {
  members = [MV_SWBD.TXA_FEED, TX_A, MSB_A.MAIN_A];
  note "87T zone concept; CTs at MV feeder and LV main.";
}

// ---------- Operating states ----------
state "normal" {
  CB_IA     = closed;
  CB_IB     = closed;
  TIE_MV    = open;
  CB_TIE_LV = open;
  ATS1      = in1;
}

state "MV incomer A maintenance" {
  use "normal";
  CB_IA  = open;
  TIE_MV = closed;
}

state "TX_A outage, LV tie closed" {
  use "normal";
  MAIN_A    = open;
  CB_TIE_LV = closed;
}

// ---------- Interlocks ----------
interlock "no paralleling of independent MV sources" {
  require !(TIE_MV = closed && CB_IA = closed && CB_IB = closed);
}

interlock "ATS never parallels inputs" {
  require !(ATS1 = both);
}

interlock "LV bus tie only under single-transformer operation" {
  require !(CB_TIE_LV = closed) || !(MAIN_A = closed && MAIN_B = closed);
}

// ---------- Scenarios ----------
scenario "Loss of grid incomer A from normal" {
  use "normal";
  set GRID_A.status = offline;

  expect de_energized(MSB_A);
  expect energized(ESSENTIALS);      // via ATS1 transfer to GEN1
  expect islanded(GEN1);
}

scenario "TX_A outage with LV tie closed" {
  use "TX_A outage, LV tie closed";
  expect energized(MSB_A);           // fed from MSB_B via CB_TIE_LV
  expect energized(ESSENTIALS);
}

layout {
  rank = source_to_load;
  flow = top_to_bottom;
  MV_SWBD     { orientation = horizontal; busbar = top; group_by = section; }
  MSB_A       { column = 1; }
  MSB_B       { column = 3; }
  ESSENTIALS  { column = 2; }
}
```

Reading notes:

- Both MV incomers feed their own bus section; the tie is normally open. Section-level `incomers` gate external feeds (R-110).
- `PUMP_WATER`'s circuit shows the full breaker → contactor → cable → motor expansion (§9.3).
- GEN1 is `duty = standby`, so it becomes available to ATS1 only when the higher-priority input is dead (§10.4).
- The interlocks are the plant's operating discipline, now machine-checked against every state (R-603) and paralleling rule (R-410).

### 18.2 Commercial site: PV + battery at LV, export-limited

```
profile "esld/1.0";

voltsys LV = 400V, 3ph, 50Hz;

code "local DSO" { export_limit_kw = 30kW; };

grid GRID : grid { vs = LV; supply_a = 100A; fault_mva = 10MVA; export_allowed = yes; };

pv_array PV1 : pv_array  { kw_peak = 40kW; strings = 8; voc = 480V; isc = 30A; };
battery BATT1 : battery { kwh = 60kWh; kw_charge = 30kW; kw_discharge = 30kW; chemistry = LFP; soc_min = 10%; };

inverter INV1 : inverter {
  kind = hybrid; kw = 30kW; vs = LV;
  island_capable = yes; export_limit = 30kW; transfer_ms = 20ms;
}

board MSB : board {
  vs = LV;
  incomers = [GRID.out, INV1.ac_out];
  priority = [GRID, INV1];
  busbar_rating_a = 250A;

  main_switch MAIN : main_switch { rating_a = 100A; poles = 4; }
  meter KWh1 : meter { kind = bidirectional; }

  circuit PV_FEED {
    protection : mccb { rating_a = 63A; breaking_ka = 10kA; };
    cable      = { csa = 16mm2; cores = 5; };
  }

  circuit LIGHTS {
    protection : rcbo { rating_a = 20A; curve = C; rcd_ma = 30mA; };
    loads      = [LIGHT_ZONE_1];
  }

  circuit SOCKETS {
    protection : rcbo { rating_a = 32A; curve = C; rcd_ma = 30mA; };
    loads      = [SOCK_ZONE_1];
  }
}

connect GRID.out   -> MSB.MAIN.in;
connect MSB.MAIN.out -> MSB.bus;
connect INV1.ac_out -> MSB.PV_FEED.out;   // backfeed onto bus via PV_FEED
connect PV1.out    -> INV1.dc_in;
connect BATT1.dc_bidi -> INV1.dc_in;

scenario "Midday export" {
  set GRID.status    = online;
  set PV1.irradiance = 1000W/m2;
  set BATT1.soc      = 100%;
  expect power_at(GRID.out) >= -30kW;     // export ≤ 30 kW
}
```

---

## 19. Extension mechanism

### 19.1 Custom types

Any project MAY declare types via `type`. Custom types MUST declare ports and params. They MUST NOT shadow built-in type names without a namespace prefix.

```
type smart_relay : switch {
  extends control_relay;
  ports {
    in   : ac_in;
    out  : ac_out;
    coil : signal;
  }
  params {
    protocol : enum(zigbee, wifi, zwave) required;
    rating_a : quantity(A) = 16A;
  }
};
```

### 19.2 Custom properties

Unknown properties on any node MUST be preserved. Validators MUST ignore unknown properties unless a code profile references them.

### 19.3 Vendor namespaces

```
vendor "acme" {
  type feeder_management : relay { ... };
  rule "ACME-001" { ... };
}
```

Vendor types are referenced as `acme.feeder_management`. Vendor rules are code-profile rules under a namespace; their IDs are prefixed (`ACME-001`).

### 19.4 Reserved for future versions

- `simulate` blocks for numeric load flow and fault calculations
- Protection curve data and coordination checking against time-current curves
- `tariff` and `control` blocks for dispatch optimization
- `thermal` blocks for cable derating and transformer loading cycles
- Arc-flash incident-energy data blocks
- `cost` and `bom` blocks for quoting
- Import/export of IEC 61850, CIM, and OpenDSS models
- Multi-diagram projects (SLD plus schematics plus wiring)

---

## 20. Roadmap

| Version | Content |
|---|---|
| **0.1** | This document: grammar, IR, built-in types, states/interlocks, structural + source rules, scenarios |
| **0.2** | Full rule catalog per code profile; coordination data model; phase imbalance numerics |
| **0.3** | Layout solver contract; deterministic SVG baseline; symbol library registry |
| **0.4** | Round-trip conformance test suite; JSON Schema for the IR |
| **0.5** | Numeric load-flow hook (`simulate`); fault-current data model |
| **0.6** | Bidirectional import from common CAD/EDA exports |
| **1.0** | Frozen grammar, frozen IR, published conformance suite |

---

## 21. Open questions for review

1. Should `zone` become normative (affecting validation, e.g. overlap/completeness checks) rather than advisory?
2. Should interlocks support time (e.g. "permitted for ≤ 200 ms during closed-transition transfer")?
3. How should per-phase switching (single-pole devices, phase-splitting at terminals) be expressed — connection phase lists, or per-pole sub-devices?
4. Is the `standby` availability rule (§10.4) sufficient, or do scenarios need explicit generator start sequencing?
5. How much relay settings structure should be schema'd before 1.0, given vendors own the semantics?
6. DC microgrids: are `bus`/`tie` semantics with domain `dc` enough, or do bipolar (±) DC systems need first-class treatment?
7. Numeric fault levels: where do they belong — `voltsys.fault_mva`, per-node `fault_mva` overrides, or a future `simulate` block only?
8. Multi-diagram projects: one file with multiple named diagrams, or one file per diagram with a project manifest?

---

## Appendix A — Differences from RSLD v0.1

ESLD generalizes the RSLD seed ([deepseek.md](deepseek.md)). Intentional changes:

| Area | RSLD v0.1 | ESLD v0.1 | Why |
|---|---|---|---|
| Profile | `residential/1.0` | `esld/1.0` | Scope now general purpose |
| Extension / media type | `.rsld` | `.esld`, `application/vnd.esld+text` | Rename |
| Bus model | One implicit busbar per board | Explicit `bus` sections with `incomers`, `rating_a`; ties via `--` | Industrial main-tie-main, split and double bus |
| Incomers | Board-level | Board- and section-level | Per-section feeds |
| Feeding a board | `-> BOARD.in` | `-> BOARD.bus` preferred; `in` kept as alias | Clearer attachment semantics |
| Circuits | `protection` shorthand | `protection` + `controller` shorthand | Motor feeders, starters |
| `relay` | Control relay (coil) | Protection relay (ANSI functions, trips); control relay renamed `control_relay` | Utility/industrial practice |
| Measurement kinds | `passive`/`signal` | New node kind `measurement` (ct, vt, meter, sync_check) | Distinct rendering and rules |
| Voltage systems | LV only in practice | LV/MV/HV, `dc`, `earthing`, `fault_mva`, `x_r` | General voltage levels |
| Earthing | Implicit | `earthing` schemes, `earth`, `ngr` nodes, compatibility rules | Real grounding studies |
| Operating states | — | `state` blocks with device positions | Normal/maintenance/emergency checking |
| Interlocks | — | `interlock` with boolean expressions | Machine-checkable operating discipline |
| Protection zones | — | Advisory `zone` declarations | 87-zone drawing convention |
| Scenarios | `export_at` | `power_at` (signed, general), `use "state"` | Any measurement point, states |
| Rule catalog | R-1xx..R-5xx | + R-6xx (states/interlocks), new R-111..R-114, R-207/208, R-310..R-313, R-410 | New semantics |
| Type library | Residential LV | Above plus MV/HV and industrial types (breaker, mccb, transformer, ct, vt, relay, ngr, reactor, line, ...) | Scope |

RSLD documents convert mechanically: rename extension/profile, `relay`→`control_relay`, board feeds `BOARD.in`→`BOARD.bus` (alias accepted), `export_at(X)`→`power_at(X)` with sign inverted.

---

If you want to continue, the natural next artefacts are: (a) the JSON Schema for the IR, (b) a conformance test suite of ~40 documents spanning residential through utility-substation, and (c) a reference SVG symbol library keyed to IEC 60617 and IEEE 315. The companion implementation plan is [esld-implementation.md](esld-implementation.md).
