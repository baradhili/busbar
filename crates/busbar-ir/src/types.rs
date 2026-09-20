//! Built-in type registry (spec §8).
//!
//! Static data: kinds, port signatures, and parameter defaults. Port
//! optionality encodes the R-103 contract: a power-path port is "required"
//! unless marked optional (sensing, earth links, and pass-through ports
//! that legitimately dangle in composite contexts).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Source,
    Converter,
    Storage,
    Protective,
    Switch,
    Load,
    Passive,
    Container,
    Measurement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDir {
    In,
    Out,
    Bidi,
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct PortDef {
    pub name: &'static str,
    pub dir: PortDir,
    /// Optional ports never trigger R-103.
    pub optional: bool,
    /// Multi-inlet ports never trigger R-104 (e.g. a shared DC bus).
    pub multi_in: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct TypeDef {
    pub kind: NodeKind,
    pub ports: &'static [PortDef],
    /// Parameter defaults as source-text values.
    pub defaults: &'static [(&'static str, &'static str)],
}

const fn p(name: &'static str, dir: PortDir) -> PortDef {
    PortDef {
        name,
        dir,
        optional: false,
        multi_in: false,
    }
}

const fn opt(name: &'static str, dir: PortDir) -> PortDef {
    PortDef {
        name,
        dir,
        optional: true,
        multi_in: false,
    }
}

const fn multi(name: &'static str, dir: PortDir) -> PortDef {
    PortDef {
        name,
        dir,
        optional: false,
        multi_in: true,
    }
}

const IN_OUT: &[PortDef] = &[p("in", PortDir::In), p("out", PortDir::Out)];
const POLES_DEFAULT: &[(&str, &str)] = &[("poles", "1")];

macro_rules! registry {
    ($($name:literal => $kind:ident, $ports:expr, $defaults:expr;)*) => {
        pub fn lookup(name: &str) -> Option<&'static TypeDef> {
            const REGISTRY: &[(&str, TypeDef)] = &[
                $(($name, TypeDef { kind: NodeKind::$kind, ports: $ports, defaults: $defaults })),*
            ];
            REGISTRY.iter().find(|(n, _)| *n == name).map(|(_, d)| d)
        }
    };
}

registry! {
    // Sources (§8.1)
    "grid" => Source, &[p("out", PortDir::Bidi)], &[];
    "generator" => Source, &[p("out", PortDir::Out)], &[];
    "pv_array" => Source, &[p("out", PortDir::Out)], &[];
    "pv_string" => Source, &[p("out", PortDir::Out)], &[];
    "wind_turbine" => Source, &[p("out", PortDir::Out)], &[];

    // Conversion (§8.2)
    "transformer" => Converter, &[
        p("primary", PortDir::In), p("secondary", PortDir::Out),
        opt("tertiary", PortDir::Out), opt("n", PortDir::None),
    ], &[];
    "inverter" => Converter, &[
        multi("dc_in", PortDir::In), opt("ac_in", PortDir::In),
        p("ac_out", PortDir::Out), opt("backup_out", PortDir::Out),
    ], &[];
    "rectifier" => Converter, IN_OUT, &[];
    "ups" => Converter, &[
        p("ac_in", PortDir::In), opt("bypass_in", PortDir::In),
        p("ac_out", PortDir::Out), p("batt", PortDir::Bidi),
    ], &[];

    // Storage and DC (§8.3)
    "battery" => Storage, &[p("dc_bidi", PortDir::Bidi)], &[];
    "bms" => Passive, &[opt("signal", PortDir::None)], &[];
    "dc_load" => Load, &[opt("in", PortDir::In)], &[];

    // Switchgear and switching (§8.4)
    "breaker" => Switch, &[p("in", PortDir::In), p("out", PortDir::Out), opt("trip", PortDir::None)], &[];
    "disconnector" => Switch, IN_OUT, &[];
    "load_break_switch" => Switch, IN_OUT, &[];
    "earth_switch" => Switch, &[p("in", PortDir::None)], &[];
    "contactor" => Switch, &[p("in", PortDir::In), p("out", PortDir::Out), opt("coil", PortDir::None)], &[];
    "control_relay" => Switch, &[opt("in", PortDir::In), opt("out", PortDir::Out), opt("coil", PortDir::None)], &[];
    "ats" => Switch, &[p("in1", PortDir::In), p("in2", PortDir::In), p("out", PortDir::Out)], &[];
    "changeover" => Switch, &[p("in1", PortDir::In), p("in2", PortDir::In), p("out", PortDir::Out)], &[];
    "main_switch" => Switch, IN_OUT, &[];
    "isolator" => Switch, IN_OUT, &[];

    // Protection devices (§8.5)
    "mcb" => Protective, IN_OUT, POLES_DEFAULT;
    "mccb" => Protective, IN_OUT, POLES_DEFAULT;
    "rcbo" => Protective, IN_OUT, POLES_DEFAULT;
    "rcd" => Protective, IN_OUT, POLES_DEFAULT;
    "afci" => Protective, IN_OUT, POLES_DEFAULT;
    "fuse" => Protective, IN_OUT, POLES_DEFAULT;

    // Measurement and protection systems (§8.6)
    "spd" => Protective, &[p("in", PortDir::In), opt("pe", PortDir::None)], &[];
    "ct" => Measurement, &[opt("signal", PortDir::None)], &[];
    "vt" => Measurement, &[opt("signal", PortDir::None)], &[];
    "sync_check" => Measurement, &[opt("signal", PortDir::None)], &[];
    "meter" => Measurement, &[opt("in", PortDir::In), opt("out", PortDir::Out)], &[];
    "relay" => Protective, &[opt("signal", PortDir::None)], &[];

    // Passive distribution and grounding (§8.7)
    "bus" => Passive, &[], &[];
    "busbar" => Passive, &[], &[];
    "junction" => Passive, &[opt("a", PortDir::Bidi), opt("b", PortDir::Bidi), opt("c", PortDir::Bidi), opt("d", PortDir::Bidi)], &[];
    "cable" => Passive, &[p("a", PortDir::Bidi), p("b", PortDir::Bidi)], &[];
    "line" => Passive, &[p("a", PortDir::Bidi), p("b", PortDir::Bidi)], &[];
    "ngr" => Passive, &[p("a", PortDir::Bidi), p("b", PortDir::Bidi)], &[];
    "reactor" => Passive, &[p("a", PortDir::Bidi), p("b", PortDir::Bidi)], &[];
    "earth" => Passive, &[p("e", PortDir::None)], &[];
    "capacitor_bank" => Load, &[opt("in", PortDir::In)], &[];

    // Loads (§8.8) — load `in` ports are optional: an unfed load is R-105
    // (warning), not R-103; dual feeds are caught by R-104.
    "load" => Load, &[opt("in", PortDir::In)], &[];
    "lighting" => Load, &[opt("in", PortDir::In)], &[];
    "socket" => Load, &[opt("in", PortDir::In)], &[];
    "appliance" => Load, &[opt("in", PortDir::In)], &[];
    "heating" => Load, &[opt("in", PortDir::In)], &[];
    "oven" => Load, &[opt("in", PortDir::In)], &[];
    "cooktop" => Load, &[opt("in", PortDir::In)], &[];
    "hws" => Load, &[opt("in", PortDir::In)], &[];
    "hvac" => Load, &[opt("in", PortDir::In)], &[];
    "pool_pump" => Load, &[opt("in", PortDir::In)], &[];
    "evse" => Load, &[opt("in", PortDir::In)], &[];
    "motor" => Load, &[opt("in", PortDir::In)], &[];

    // Containers (§8.9) — board ports are attachment points, all optional.
    "board" => Container, &[
        opt("in", PortDir::In), opt("out", PortDir::Out),
        opt("n", PortDir::None), opt("pe", PortDir::None), opt("bus", PortDir::Bidi),
    ], &[];
}

impl TypeDef {
    pub fn port(&self, name: &str) -> Option<PortDef> {
        self.ports.iter().copied().find(|p| p.name == name)
    }

    /// Effective value of a parameter: the node's property if set, else
    /// the type default, as source text.
    pub fn param<'a>(
        &self,
        props: &'a [busbar_syntax::ast::Property],
        name: &str,
    ) -> Option<&'a str> {
        if let Some(prop) = props.iter().find(|p| p.name == name) {
            return match &prop.value.value {
                busbar_syntax::ast::Value::Number(n) => Some(n.as_str()),
                busbar_syntax::ast::Value::Quantity { number, .. } => Some(number.as_str()),
                busbar_syntax::ast::Value::Ident(s) => Some(s.as_str()),
                _ => None,
            };
        }
        self.defaults
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
    }
}
