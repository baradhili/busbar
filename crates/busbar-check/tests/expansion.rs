//! Circuit expansion connectivity (spec §9.3): the busbar feeds
//! protection when present, else the controller; loads hang off the
//! controller when one exists. The controller-only case regressed
//! silently once — these tests keep it pinned.

use busbar_check::Severity;

fn errors_and_unreachables(src: &str) -> (Vec<String>, Vec<String>) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let diags = busbar_check::check(src, Some(base)).expect("parse+check");
    let mut errors = Vec::new();
    let mut unreachables = Vec::new();
    for d in diags {
        match (d.code.as_str(), d.severity) {
            ("R-103", Severity::Error) | ("R-105", _) => {
                unreachables.push(format!("{}: {}", d.code, d.message));
            }
            (_, Severity::Error) => errors.push(format!("{}: {}", d.code, d.message)),
            _ => {}
        }
    }
    (errors, unreachables)
}

const SHELL: &str = r#"
    profile "esld/1.0";
    voltsys LV = 230V, 1ph, 50Hz;
    grid GRID : grid { vs = LV; supply_a = 63A; };
    board MAIN : board {
      vs = LV;
      incomers = [GRID.out];
      busbar_rating_a = 100A;
      main_switch MSB : main_switch { rating_a = 63A; };
      circuit FAN {
        controller : contactor { rating_a = 16A; utilization = "AC-3"; };
        loads = [M1];
      };
    };
    motor M1 : motor { kw = 1.5kW; starter = dol; };
    connect GRID.out -> MAIN.MSB.in;
    connect MAIN.MSB.out -> MAIN.bus;
    "#;

#[test]
fn controller_only_circuit_is_connected() {
    // Without controller-headed expansion the contactor's in port is
    // R-103-unfed and the motor R-105-unreachable.
    let (errors, dangling) = errors_and_unreachables(SHELL);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert!(dangling.is_empty(), "unfed/unreachable: {dangling:?}");
}

#[test]
fn protection_plus_controller_stays_in_series() {
    // With both present: bus -> protection -> controller -> load.
    let src = SHELL.replace(
        "controller : contactor",
        "protection : rcbo { rating_a = 16A; };\n        controller : contactor",
    );
    let (errors, dangling) = errors_and_unreachables(&src);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert!(dangling.is_empty(), "unfed/unreachable: {dangling:?}");
}
