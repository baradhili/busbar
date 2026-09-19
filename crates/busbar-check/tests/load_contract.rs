//! The load-port contract (spec §8.8 note, types.rs): an unfed load is
//! R-105 (warning), never R-103; feeding it clears all diagnostics.
//! capacitor_bank is the regression case — it used to deviate with a
//! required port.

fn diags(source: &str) -> Vec<(String, String, u32)> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    busbar_check::check(source, Some(base))
        .expect("parse+check")
        .into_iter()
        .map(|d| {
            (
                d.code,
                match d.severity {
                    busbar_check::Severity::Error => "error".into(),
                    busbar_check::Severity::Warning => "warning".into(),
                },
                d.line,
            )
        })
        .collect()
}

#[test]
fn unfed_capacitor_bank_warns_but_never_errors() {
    let src = r#"
        profile "esld/1.0";
        voltsys LV = 230V, 1ph, 50Hz;
        capacitor_bank CAP : capacitor_bank { kvar = 50kvar; stages = 3; }
        "#;
    let found = diags(src);
    assert!(
        found.iter().any(|(c, _, _)| c == "R-105"),
        "expected an R-105 warning for the unfed bank, got {found:?}"
    );
    assert!(
        found.iter().all(|(c, s, _)| c != "R-103" || s != "error"),
        "unfed load must not produce R-103, got {found:?}"
    );
}

#[test]
fn fed_capacitor_bank_checks_clean() {
    let src = r#"
        profile "esld/1.0";
        voltsys LV = 230V, 1ph, 50Hz;
        grid GRID : grid { vs = LV; supply_a = 63A; };
        board MAIN : board {
          vs = LV;
          incomers = [GRID.out];
          busbar_rating_a = 100A;
          main_switch MSB : main_switch { rating_a = 63A; };
          circuit PFC {
            protection : rcbo { rating_a = 20A; };
            loads = [CAP];
          };
        };
        capacitor_bank CAP : capacitor_bank { kvar = 50kvar; stages = 3; };
        connect GRID.out -> MAIN.MSB.in;
        connect MAIN.MSB.out -> MAIN.bus;
        "#;
    assert_eq!(diags(src), Vec::new(), "fed bank must check clean");
}
