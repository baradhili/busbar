//! Regression tests for untrusted-input robustness (CodeRabbit findings):
//! the µ unit prefix byte-slice, end-of-input values, the `..` range
//! operator, and trailing-comment placement after statements.

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
