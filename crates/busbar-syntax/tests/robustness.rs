//! Regression tests for untrusted-input robustness (CodeRabbit findings):
//! the µ unit prefix byte-slice, end-of-input values, the `..` range
//! operator, and trailing-comment placement after statements.

#[test]
fn unit_prefixes_normalize_and_passthrough() {
    // Prefix tail is sliced by len_utf8; multipliers and unprefixed units
    // are preserved. mm2 must NOT be read as m-prefixed m2.
    let cases = [
        ("5", "kW", 5000.0, "W"),
        ("250", "mV", 0.25, "V"),
        ("2", "MVA", 2e6, "VA"),
        ("3", "kA", 3000.0, "A"),
        ("63", "A", 63.0, "A"),
        ("2.5", "mm2", 2.5, "mm2"),
        // Complete base units are never prefix-stripped: bare "m" is
        // metres, not milli-nothing; ms is still milliseconds.
        ("18", "m", 18.0, "m"),
        ("1", "s", 1.0, "s"),
        ("250", "ms", 0.25, "s"),
    ];
    for (number, unit_text, want, unit_want) in cases {
        let v = busbar_syntax::ast::Value::Quantity {
            number: number.to_owned(),
            unit: unit_text.to_owned(),
        };
        let (val, unit) = v.quantity().expect("quantity");
        assert!(
            (val - want).abs() < 1e-9,
            "{number}{unit_text}: got {val}{unit}, want {want}{unit_want}"
        );
        assert_eq!(unit, unit_want, "{number}{unit_text}");
    }
}

#[test]
fn micro_unit_prefix_does_not_panic() {
    // 'µ' is multi-byte; Value::quantity() used to slice at byte 1.
    let doc = busbar_syntax::parse("voltsys A = { nominal = 10µV; };").expect("parses");
    let busbar_syntax::ast::Statement::Voltsys { body, .. } = &doc.statements[0] else {
        panic!("unexpected statement shape");
    };
    let busbar_syntax::ast::VoltsysBody::Block(props) = body else {
        panic!("expected block voltsys body");
    };
    let v = props.iter().find(|p| p.name == "nominal").expect("nominal");
    let (val, unit) = v.value.value.quantity().expect("quantity");
    assert!((val - 1e-5).abs() < 1e-12);
    assert_eq!(unit, "V");
}

#[test]
fn unspaced_numeric_range_lexes() {
    // `1..5` must lex as Number(1), `..`, Number(5) — not one number.
    let doc = busbar_syntax::parse("group G = [1..5];").expect("range parses");
    assert!(matches!(
        &doc.statements[0],
        busbar_syntax::ast::Statement::Group { list, .. }
            if matches!(&list.value,
                busbar_syntax::ast::Value::List(items)
                    if matches!(&items[0].value, busbar_syntax::ast::Value::Range(_, _)))
    ));
}

#[test]
fn decimal_numbers_split_from_range_operator() {
    // Single decimal points stay in the number; the dot pair still forms
    // the range operator: `1.5..7.25` -> 1.5, .., 7.25.
    let doc = busbar_syntax::parse("group H = [1.5..7.25];").expect("range parses");
    let busbar_syntax::ast::Statement::Group { list, .. } = &doc.statements[0] else {
        panic!("expected group");
    };
    let busbar_syntax::ast::Value::List(items) = &list.value else {
        panic!("expected list");
    };
    let busbar_syntax::ast::Value::Range(lo, hi) = &items[0].value else {
        panic!("expected range");
    };
    let (lo_v, _) = lo.value.quantity().expect("low quantity");
    let (hi_v, _) = hi.value.quantity().expect("high quantity");
    assert!((lo_v - 1.5).abs() < 1e-12);
    assert!((hi_v - 7.25).abs() < 1e-12);
}

#[test]
fn truncated_value_is_an_error_not_a_panic() {
    let err = busbar_syntax::parse("board B { x =").expect_err("must fail");
    assert!(err.message.contains("value"), "got: {}", err.message);
}

#[test]
fn trailing_comment_stays_on_the_statement_line() {
    let src = "a = 1; // note\n";
    let out = busbar_syntax::fmt::format(src).expect("formats");
    assert_eq!(out, "a = 1; // note\n");
}

#[test]
fn trailing_comment_after_closing_brace_stays_on_its_line() {
    // A block-closing } sets the pending newline (5e6ebe2) — the comment
    // must still bind to that line, not drop to the next one.
    let src = "state \"x\" {\n  A = open;\n} // mode\n";
    let out = busbar_syntax::fmt::format(src).expect("formats");
    assert_eq!(out, "state \"x\" {\n  A = open;\n} // mode\n");
}
