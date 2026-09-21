//! Symbol fragments transcribed from the reference sheet
//! `corpus/render/symbols.svg` (SmartSLD). Each constant is the inner
//! markup of the sheet's `<g id=…>` group, normalized: stroke colour and
//! weight inherit from the parent `<g>` the renderer emits (fill none,
//! #222222, 1px) unless the mark carries a deliberate finer weight
//! (0.4/0.8/1.5), and text keeps its explicit fill so it survives the
//! `fill="none"` parent. Coordinates are centred on the glyph; leads run
//! to ±20 (fuse ±30) so wires land exactly on the sheet's lead ends.
//!
//! Derived variants: `SWITCH` is the sheet blade without a function
//! mark; `MAIN_SWITCH` is the disconnector plus the breaker's x release.

/// Returns the sheet fragment for `glyph`, if it has one; `None` keeps
/// the hand-drawn primitive (transformer, lamp, motor, spd, ats,
/// heating, junction, boards, generic).
pub fn fragment(glyph: crate::Glyph) -> Option<&'static str> {
    Some(match glyph {
        crate::Glyph::Switch => SWITCH,
        crate::Glyph::Protective => BREAKER,
        crate::Glyph::Disconnector => DISCONNECTOR,
        crate::Glyph::MainSwitch => MAIN_SWITCH,
        crate::Glyph::Contactor => CONTACTOR,
        crate::Glyph::Rcbo => RCBO,
        crate::Glyph::Rcd => RCD,
        crate::Glyph::Fuse => FUSE,
        crate::Glyph::Earth => EARTH,
        crate::Glyph::Relay => RELAY_COIL,
        crate::Glyph::Ct => CT,
        crate::Glyph::Meter => ENERGY_METER,
        crate::Glyph::Generator => GENERATOR,
        crate::Glyph::Source => SUPPLY,
        crate::Glyph::Evse => EVSE,
        crate::Glyph::Load => LOAD,
        crate::Glyph::Socket => SOCKET,
        crate::Glyph::Battery => BATTERY,
        crate::Glyph::Inverter => INVERTER,
        crate::Glyph::Pv => PV,
        crate::Glyph::WindTurbine => WIND_TURBINE,
        _ => return None,
    })
}

const SWITCH: &str = r##"<line x1="0" y1="-20" x2="0" y2="-10"/><polyline points="-5,-10 0,10 0,20"/><line x1="0" y1="6" x2="-5" y2="-10"/>"##;

const BREAKER: &str = r##"<polyline points="0,20 0,10 -5,-10"/><line x1="0" y1="-20" x2="0" y2="-10"/><line x1="2" y1="-8" x2="-2" y2="-12"/><line x1="2" y1="-12" x2="-2" y2="-8"/>"##;

const DISCONNECTOR: &str = r##"<line x1="0" y1="-20" x2="0" y2="-23"/><line x1="0" y1="0" x2="0" y2="20"/><line x1="0" y1="0" x2="-8" y2="-23"/><line x1="-3" y1="-20" x2="3" y2="-20"/>"##;

const MAIN_SWITCH: &str = r##"<line x1="0" y1="-20" x2="0" y2="-23"/><line x1="0" y1="0" x2="0" y2="20"/><line x1="0" y1="0" x2="-8" y2="-23"/><line x1="-3" y1="-20" x2="3" y2="-20"/><line x1="2" y1="-8" x2="-2" y2="-12"/><line x1="2" y1="-12" x2="-2" y2="-8"/>"##;

const CONTACTOR: &str = r##"<line x1="0" y1="-20" x2="0" y2="-23"/><line x1="0" y1="0" x2="0" y2="20"/><line x1="0" y1="0" x2="-8" y2="-23"/><path d="M 0 -24.5 A 2.5 2.5 0 0 0 0 -19.5"/>"##;

const RCBO: &str = r##"<text x="-14" y="13.8" font-size="6" fill="#222222">IΔ</text><line x1="2" y1="-12" x2="-2" y2="-8"/><line x1="-2" y1="-12" x2="2" y2="-8"/><polyline points="-5,-10 0,10 0,20"/><line x1="0" y1="-20" x2="0" y2="-10"/><line x1="-4" y1="-4" x2="-8" y2="-4" stroke-width="0.4"/><line x1="-8" y1="-4" x2="-8" y2="-8" stroke-width="0.4"/><line x1="-13" y1="-8" x2="-8" y2="-8" stroke-width="0.4"/><line x1="-13" y1="-8" x2="-13" y2="-4" stroke-width="0.4"/><line x1="-16" y1="-4" x2="-13" y2="-4" stroke-width="0.4"/><path d="M -8 1 A 2.5 3.5 0 0 0 -13 1" stroke-width="0.4"/><line x1="-8" y1="1" x2="-2" y2="1" stroke-width="0.4"/><line x1="-16" y1="1" x2="-13" y2="1" stroke-width="0.4"/>"##;

const RCD: &str = r##"<line x1="0" y1="-20" x2="0" y2="-10"/><line x1="2" y1="-12" x2="-2" y2="-8"/><line x1="-2" y1="-12" x2="2" y2="-8"/><polyline points="0,20 0,10 -7,-9"/><ellipse cx="0" cy="13" rx="4.5" ry="2"/><polyline points="-4,0 -10,0 -10,13 -5,13" stroke-width="0.4" stroke-dasharray="4 2"/>"##;

const FUSE: &str = r##"<line x1="0" y1="-30" x2="0" y2="30"/><rect x="-5" y="-15" width="10" height="30" rx="0" ry="0"/>"##;

const EARTH: &str = r##"<line x1="-10" y1="0" x2="10" y2="0"/><line x1="0" y1="-15" x2="0" y2="0"/><line x1="-7" y1="5" x2="7" y2="5"/><line x1="-4" y1="10" x2="4" y2="10"/><line x1="0.5" y1="15" x2="-0.5" y2="15"/>"##;

const RELAY_COIL: &str = r##"<rect x="-20" y="-10" width="40" height="20" rx="0" ry="0"/><line x1="0" y1="-20" x2="0" y2="-10"/><line x1="0" y1="10" x2="0" y2="20"/>"##;

const CT: &str = r##"<g transform="translate(30 30)\"><line x1="-30" y1="-60" x2="-30" y2="0"/><line x1="-20" y1="-30" x2="10" y2="-30"/><ellipse cx="-30" cy="-30" rx="10" ry="10"/><line x1="-10" y1="-20" x2="0" y2="-40"/><line x1="-5" y1="-20" x2="5" y2="-40"/></g>"##;

const ENERGY_METER: &str = r##"<text x="-10" y="19.2" font-size="9" fill="#222222">Wh</text><rect x="-20" y="-10" width="40" height="40" rx="0" ry="0"/><rect x="-20" y="-30" width="40" height="20" rx="0" ry="0"/>"##;

const GENERATOR: &str = r##"<g transform="translate(20 20)\"><rect x="-40" y="-40" width="40" height="40" rx="0" ry="0"/><text x="-24" y="-8.8" font-size="14" fill="#222222">G</text></g>"##;

const SUPPLY: &str = r##"<circle cx="0" cy="-15" r="14"/><text x="0" y="-12" text-anchor="middle" font-size="14" fill="#222222">~</text><line x1="0" y1="-1" x2="0" y2="20"/>"##;

const EVSE: &str = r##"<line x1="0" y1="-25" x2="0" y2="-15"/><rect x="-15" y="-15" width="30" height="35"/><rect x="-9" y="-8" width="14" height="6" stroke-width="0.8"/><rect x="5" y="-7" width="2" height="4" fill="#222222"/><text x="-9" y="13" font-size="7" fill="#222222">EV</text>"##;

const LOAD: &str =
    r##"<line x1="0" y1="-20" x2="0" y2="10"/><polygon points="-6,10 6,10 0,22" fill="#222222"/>"##;

const SOCKET: &str = r##"<line x1="-5" y1="13" x2="4" y2="20" stroke-width="0.4"/><line x1="-1" y1="12" x2="-1" y2="19"/><line x1="0" y1="12" x2="0" y2="30"/><line x1="0" y1="9" x2="0" y2="0"/><line x1="1" y1="12" x2="1" y2="19"/><path d="M 3 13 A 3 3 0 0 0 -3 13"/>"##;

const BATTERY: &str = r##"<line x1="-10" y1="0" x2="0" y2="0"/><line x1="0" y1="-5" x2="0" y2="5"/><line x1="5" y1="-10" x2="5" y2="10"/><line x1="5" y1="0" x2="15" y2="0"/>"##;

const INVERTER: &str = r##"<g transform="matrix(1 0 0 -1 0 -40)"><path d="M -15 -8 A 5 2.5 0 0 0 -25 -8"/><line x1="-40" y1="0" x2="0" y2="-20"/><path d="M -15 -8 A 5 2.5 0 0 0 -5 -8"/><rect x="-40" y="-40" width="40" height="40" rx="0" ry="0"/><line x1="-36" y1="-36" x2="-20" y2="-36"/><line x1="-36" y1="-32" x2="-30" y2="-32"/><line x1="-26" y1="-32" x2="-20" y2="-32"/></g>"##;

const PV: &str = r##"<g transform="translate(-20 -10)\"><line x1="-16" y1="24" x2="-26" y2="14"/><line x1="-21" y1="29" x2="-31" y2="19"/><polygon points="-20,23 -17,20 -15,25" fill="#222222"/><polygon points="-22,25 -25,28 -20,30" fill="#222222"/><rect x="-10" y="-20" width="60" height="60" rx="0" ry="0"/><text x="15" y="7.2" font-size="14" fill="#222222">G</text><line x1="10" y1="20" x2="20" y2="20"/><line x1="20" y1="15" x2="20" y2="25"/><line x1="25" y1="10" x2="25" y2="30"/><line x1="25" y1="20" x2="35" y2="20"/></g>"##;

const WIND_TURBINE: &str = r##"<circle cx="0" cy="0" r="20"/><text x="0" y="4" text-anchor="middle" font-size="16" fill="#222222">G</text><line x1="0" y1="-20" x2="0" y2="-32"/><line x1="0" y1="-32" x2="-10" y2="-39" stroke-width="1.5"/><line x1="0" y1="-32" x2="10" y2="-39" stroke-width="1.5"/><line x1="0" y1="-32" x2="0" y2="-44" stroke-width="1.5"/>"##;
