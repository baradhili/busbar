//! Cucumber acceptance harness — run with `cargo test --test cucumber`.
//!
//! All implemented features run by default. Scenarios for areas whose
//! milestones have not landed are tagged `@incomplete` and excluded from
//! the default run and CI; set `BUSBAR_INCOMPLETE=1` to include them —
//! they fail honestly (undefined steps or missing fixtures) until their
//! milestone lands, at which point the tag is removed.
//!
//! Feature files live in `features/`; corpus documents in `corpus/`.

use std::fmt;
use std::path::PathBuf;

use cucumber::World;

/// Repository root, derived from this test target's manifest.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"));

/// Severity of a [`Diagnostic`], mirroring the CLI JSON schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
        })
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "error" => Ok(Self::Error),
            "warning" => Ok(Self::Warning),
            other => Err(format!(
                "unknown severity {other:?}, expected error|warning"
            )),
        }
    }
}

/// One validation finding, mirroring the toolchain diagnostic shape
/// (implementation plan §4.4). Line is 1-based.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Diagnostic {
    code: String,
    severity: Severity,
    line: u32,
}

/// State carried between steps.
#[derive(Debug, Default, World)]
struct BusbarWorld {
    path: Option<PathBuf>,
    source: Option<String>,
    parse: Option<Result<(), String>>,
    check: Option<Result<Vec<Diagnostic>, String>>,
    fmt_first: Option<Result<String, String>>,
    fmt_second: Option<Result<String, String>>,
    /// Diagnostic asserted by the last "it reports" step, so the
    /// following "no other errors" step excludes exactly it.
    expected: Option<Diagnostic>,
}

// -- Toolchain seam --------------------------------------------------------
// Thin delegations to the implementation crates. R-3xx/R-4xx scenarios
// stay commented out until M4; solver steps delegate to busbar-solve at M6.

fn parse(source: &str) -> Result<(), String> {
    busbar_syntax::parse_or_string(source).map(|_| ())
}

fn check(source: &str, base_dir: Option<&std::path::Path>) -> Result<Vec<Diagnostic>, String> {
    busbar_check::check(source, base_dir).map(|diags| {
        diags
            .into_iter()
            .map(|d| Diagnostic {
                code: d.code,
                severity: match d.severity {
                    busbar_check::Severity::Error => Severity::Error,
                    busbar_check::Severity::Warning => Severity::Warning,
                },
                line: d.line,
            })
            .collect()
    })
}

fn format_once(source: &str) -> Result<String, String> {
    busbar_syntax::fmt::format(source).map_err(|e| format!("line {}: {}", e.line, e.message))
}

fn asts_equal(original: &str, formatted: &str) -> Result<bool, String> {
    let a = busbar_syntax::fmt::significant_tokens(original)
        .map_err(|e| format!("line {}: {}", e.line, e.message))?;
    let b = busbar_syntax::fmt::significant_tokens(formatted)
        .map_err(|e| format!("line {}: {}", e.line, e.message))?;
    Ok(a == b)
}

/// Panics with the pending-operation cause when the seam is not
/// implemented — this keeps the not-yet-implemented scenarios honestly red.
fn unwrap_or_panic<T>(result: &Result<T, String>) -> &T {
    match result {
        Ok(value) => value,
        Err(cause) => panic!("pending implementation: {cause}"),
    }
}

fn source_of(world: &BusbarWorld) -> &str {
    world.source.as_deref().expect("no document loaded")
}

// -- Steps ------------------------------------------------------------------

#[cucumber::given(regex = r#"^the document "([^"]+)"$"#)]
async fn the_document(world: &mut BusbarWorld, rel: String) {
    let path = PathBuf::from(ROOT).join("corpus").join(&rel);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read corpus file {}: {e}", path.display()));
    world.path = Some(path);
    world.source = Some(source);
    world.parse = None;
    world.check = None;
    world.fmt_first = None;
    world.fmt_second = None;
    world.expected = None;
}

#[cucumber::when(regex = r"^I parse it$")]
async fn parse_it(world: &mut BusbarWorld) {
    let source = source_of(world).to_owned();
    world.parse = Some(parse(&source));
}

#[cucumber::when(regex = r"^I check it$")]
async fn check_it(world: &mut BusbarWorld) {
    let source = source_of(world).to_owned();
    let dir = world
        .path
        .as_ref()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    world.check = Some(check(&source, dir.as_deref()));
}

#[cucumber::when(regex = r"^I format it$")]
async fn format_it(world: &mut BusbarWorld) {
    if world.fmt_first.is_none() {
        let source = source_of(world).to_owned();
        world.fmt_first = Some(format_once(&source));
    } else {
        let first = match world.fmt_first.clone().expect("first formatting missing") {
            Ok(text) => format_once(&text),
            Err(cause) => Err(cause),
        };
        world.fmt_second = Some(first);
    }
}

#[cucumber::then(regex = r"^it parses without error$")]
async fn parses_without_error(world: &mut BusbarWorld) {
    let result = world.parse.as_ref().expect("`When I parse it` not run");
    unwrap_or_panic(result);
}

#[cucumber::then(regex = r"^there are no error diagnostics$")]
async fn no_error_diagnostics(world: &mut BusbarWorld) {
    let diagnostics = unwrap_or_panic(world.check.as_ref().expect("`When I check it` not run"));
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no error diagnostics, got {errors:?}"
    );
}

#[cucumber::then(regex = r#"^it reports "([^"]+)" with severity (error|warning) at line (\d+)$"#)]
async fn reports_rule(world: &mut BusbarWorld, code: String, severity: Severity, line: u32) {
    let diagnostics = unwrap_or_panic(world.check.as_ref().expect("`When I check it` not run"));
    let found = diagnostics
        .iter()
        .find(|d| d.severity == severity && d.code == code && d.line == line);
    assert!(
        found.is_some(),
        "expected {severity} {code} at line {line} of {}, got {diagnostics:?}",
        world.path.as_ref().expect("no path").display()
    );
    world.expected = Some(Diagnostic {
        code,
        severity,
        line,
    });
}

#[cucumber::then(regex = r"^it reports no other errors$")]
async fn reports_no_other_errors(world: &mut BusbarWorld) {
    let expected = world
        .expected
        .clone()
        .expect("`it reports` step not run first");
    let diagnostics = unwrap_or_panic(world.check.as_ref().expect("`When I check it` not run"));
    let others: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error && **d != expected)
        .collect();
    assert!(
        others.is_empty(),
        "expected {expected:?} to be the only error, also got {others:?}"
    );
}

#[cucumber::then(regex = r"^both formatted outputs are identical$")]
async fn formatted_outputs_identical(world: &mut BusbarWorld) {
    let first = unwrap_or_panic(world.fmt_first.as_ref().expect("first format missing"));
    let second = unwrap_or_panic(world.fmt_second.as_ref().expect("second format missing"));
    assert_eq!(first, second, "formatter is not idempotent");
}

#[cucumber::then(regex = r"^parsing the formatted output yields an AST equal to the original's$")]
async fn formatted_ast_equal(world: &mut BusbarWorld) {
    let original = source_of(world);
    let formatted = unwrap_or_panic(world.fmt_first.as_ref().expect("first format missing"));
    if formatted != original {
        let comparison = asts_equal(original, formatted);
        let equal = *unwrap_or_panic(&comparison);
        assert!(equal, "formatting changed the AST");
    }
}

// Smoke-test steps (always active).
#[cucumber::given(regex = r"^the BusBar workspace is scaffolded$")]
async fn workspace_scaffolded(_world: &mut BusbarWorld) {}

#[cucumber::then(regex = r"^this scenario passes$")]
async fn scenario_passes(_world: &mut BusbarWorld) {}

#[tokio::main]
async fn main() {
    // gherkin does not propagate feature-level tags to scenarios, so the
    // filter checks both levels (learned the hard way).
    // fail_on_skipped: an undefined step is a failure, never a silent
    // skip, so the opt-in run is honestly red while areas are incomplete.
    let incomplete = std::env::var_os("BUSBAR_INCOMPLETE").is_some_and(|v| v != "0");
    let filter = |feature: &cucumber::gherkin::Feature,
                  _rule: Option<&cucumber::gherkin::Rule>,
                  scenario: &cucumber::gherkin::Scenario| {
        let tagged = || {
            feature
                .tags
                .iter()
                .chain(scenario.tags.iter())
                .any(|t| t == "incomplete")
        };
        !tagged()
    };
    if incomplete {
        BusbarWorld::cucumber()
            .fail_on_skipped()
            .run_and_exit("features")
            .await;
    } else {
        BusbarWorld::cucumber()
            .fail_on_skipped()
            .filter_run_and_exit("features", filter)
            .await;
    }
}
