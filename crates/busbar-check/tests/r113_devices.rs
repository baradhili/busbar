//! R-113 on multi-section boards (spec §9.2): power-path devices without
//! connections are flagged; measurement/relay devices are exempt because
//! they associate via signal links — the §18.1 MV_SWBD shape.

fn codes_at(src: &str, code: &str) -> Vec<u32> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    busbar_check::check(src, Some(base))
        .expect("parse+check")
        .into_iter()
        .filter(|d| d.code == code)
        .map(|d| d.line)
        .collect()
}

const SHELL_HEAD: &str = r#"
    profile "esld/1.0";
    voltsys LV = 230V, 1ph, 50Hz;
    grid GRID : grid { vs = LV; supply_a = 63A; };
    generator GEN1 : generator { vs = LV; kva = 5kVA; duty = standby; };
    board MAIN : board {
      vs = LV;
      bus A { incomers = [GRID.out]; rating_a = 100A; }
      bus B { incomers = [GEN1.out]; rating_a = 100A; }
      isolator SW_A : isolator { rating_a = 63A; };
      isolator SW_B : isolator { rating_a = 63A; };
      circuit FEED {
        bus = A;
        protection : rcbo { rating_a = 10A; };
        loads = [LAMP];
      };
    };
    lighting LAMP : lighting { kw = 0.1kW; };
    connect GRID.out -> MAIN.SW_A.in;
    connect MAIN.SW_A.out -> MAIN.A;
    connect GEN1.out -> MAIN.SW_B.in;
    connect MAIN.SW_B.out -> MAIN.B;
    "#;

#[test]
fn unconnected_power_device_in_multi_section_board_is_r113() {
    // An unconnected main switch needs busbar power: R-113 at its line
    // (line 12 in SHELL_HEAD's layout once inserted after SW_B).
    let src = SHELL_HEAD.replace(
        "isolator SW_B : isolator { rating_a = 63A; };",
        "isolator SW_B : isolator { rating_a = 63A; };\n      main_switch ORPHAN_SW : main_switch { rating_a = 63A; };",
    );
    let lines = codes_at(&src, "R-113");
    assert!(
        lines.contains(&12),
        "expected R-113 at the orphan switch (line 12), got {lines:?}"
    );
}

#[test]
fn measurement_and_relay_devices_are_exempt() {
    // §18.1 shape: CTs, VT, and relays declared unconnected on the
    // multi-section board — never R-113.
    let src = SHELL_HEAD.replace(
        "isolator SW_B : isolator { rating_a = 63A; };",
        "isolator SW_B : isolator { rating_a = 63A; };\n      ct CT_IA : ct { primary_a = 400A; secondary_a = 5A; }\n      vt VT_A : vt { primary_v = 11000V; secondary_v = 110V; kind = inductive; }\n      relay PROT_IA : relay { functions = [\"50\", \"51\"]; trips = [SW_A]; ct = CT_IA; }",
    );
    assert!(
        codes_at(&src, "R-113").is_empty(),
        "measurement/relay devices must not trigger R-113"
    );
}
