//! Cucumber acceptance harness — run with `cargo test --test cucumber`.
//!
//! All implemented features run by default. Scenarios for areas whose
//! milestones have not landed are tagged `@incomplete` and excluded from
//! the default run and CI; set `BUSBAR_INCOMPLETE=1` to include them —
//! they fail honestly (undefined steps or missing fixtures) until their
//! milestone lands, at which point the tag is removed.
//!
//! Feature files live in `features/`; corpus documents in `corpus/`.

use std::collections::BTreeMap;
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
    render_first: Option<Result<String, String>>,
    render_second: Option<Result<String, String>>,
    layout: Option<Result<(busbar_ir::Ir, busbar_layout::Layout), String>>,
    cli: Option<(i32, String, String)>,
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

/// Parses and lowers to the IR, then builds the deterministic layout
/// (spec §16.1). Both are kept: the invariants cross-reference them.
fn lay_out(source: &str) -> Result<(busbar_ir::Ir, busbar_layout::Layout), String> {
    let doc =
        busbar_syntax::parse(source).map_err(|e| format!("line {}: {}", e.line, e.message))?;
    let ir = busbar_ir::Ir::build(&doc).map_err(|e| format!("line {}: {}", e.line, e.message))?;
    let layout = busbar_layout::build(&ir);
    Ok((ir, layout))
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
    world.render_first = None;
    world.render_second = None;
    world.layout = None;
    world.cli = None;
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

#[cucumber::then(regex = r"^there are no diagnostics$")]
async fn no_diagnostics(world: &mut BusbarWorld) {
    let diagnostics = unwrap_or_panic(world.check.as_ref().expect("`When I check it` not run"));
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics at all, got {diagnostics:?}"
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

#[cucumber::when(regex = r#"^I render it with symbols "([^"]+)"$"#)]
async fn render_it(world: &mut BusbarWorld, symbols: String) {
    let source = source_of(world).to_owned();
    world.render_first = Some(busbar_render::render_str(&source, &symbols));
}

#[cucumber::when(regex = r"^I render it again$")]
async fn render_it_again(world: &mut BusbarWorld) {
    let source = source_of(world).to_owned();
    world.render_second = Some(busbar_render::render_str(&source, "iec"));
}

#[cucumber::then(regex = r#"^the SVG hash is "([0-9a-f]{64})"$"#)]
async fn svg_hash_is(world: &mut BusbarWorld, expected: String) {
    let svg = unwrap_or_panic(world.render_first.as_ref().expect("render not run"));
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(svg.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, expected, "golden SVG hash mismatch");
}

#[cucumber::then(regex = r"^both rendered outputs are byte-identical$")]
async fn rendered_outputs_identical(world: &mut BusbarWorld) {
    let first = unwrap_or_panic(world.render_first.as_ref().expect("first render missing"));
    let second = unwrap_or_panic(world.render_second.as_ref().expect("second render missing"));
    assert_eq!(first, second, "renderer is not byte-deterministic");
}

#[cucumber::then(regex = r"^the render succeeds$")]
async fn render_succeeds(world: &mut BusbarWorld) {
    unwrap_or_panic(world.render_first.as_ref().expect("render not run"));
}

#[cucumber::then(regex = r#"^the render fails mentioning "([^"]+)"$"#)]
async fn render_fails_mentioning(world: &mut BusbarWorld, needle: String) {
    let result = world.render_first.as_ref().expect("render not run");
    match result {
        Ok(svg) => panic!("expected render failure, got {} bytes", svg.len()),
        Err(cause) => assert!(
            cause.contains(&needle),
            "failure {cause:?} does not mention {needle:?}"
        ),
    }
}

// -- Layout steps (features/layout.feature) --------------------------------
// The drafting contract of layout-guidance §6 / spec §16.1, asserted over
// the corpus. Geometry tolerance matches busbar-layout/tests/invariants.rs.

/// Tolerance for layout geometry: places are laid out on a float grid and
/// compared with a half-cell slack, exactly like the unit invariant suite.
const LAYOUT_EPS: f64 = 0.5;

fn layout_of(world: &BusbarWorld) -> &(busbar_ir::Ir, busbar_layout::Layout) {
    unwrap_or_panic(world.layout.as_ref().expect("`When I lay it out` not run"))
}

fn places_overlap(a: &busbar_layout::Place, b: &busbar_layout::Place) -> bool {
    a.x + LAYOUT_EPS < b.x + b.w - LAYOUT_EPS
        && b.x + LAYOUT_EPS < a.x + a.w - LAYOUT_EPS
        && a.y + LAYOUT_EPS < b.y + b.h - LAYOUT_EPS
        && b.y + LAYOUT_EPS < a.y + a.h - LAYOUT_EPS
}

#[cucumber::when(regex = r"^I lay it out$")]
async fn lay_it_out(world: &mut BusbarWorld) {
    let source = source_of(world).to_owned();
    world.layout = Some(lay_out(&source));
}

#[cucumber::then(regex = r"^no glyphs overlap$")]
async fn no_glyphs_overlap(world: &mut BusbarWorld) {
    let (_, layout) = layout_of(world);
    let cells: Vec<(&String, &busbar_layout::Place)> = layout.places.iter().collect();
    for i in 0..cells.len() {
        for j in i + 1..cells.len() {
            // A board frame legitimately intersects its members; only
            // same-category pairs (board/board, glyph/glyph) must not
            // overlap each other.
            let board_i = cells[i].1.glyph == busbar_layout::Glyph::Board;
            let board_j = cells[j].1.glyph == busbar_layout::Glyph::Board;
            if board_i != board_j {
                continue;
            }
            assert!(
                !places_overlap(cells[i].1, cells[j].1),
                "`{}` overlaps `{}` at ({:.1},{:.1}) vs ({:.1},{:.1})",
                cells[i].0,
                cells[j].0,
                cells[i].1.x,
                cells[i].1.y,
                cells[j].1.x,
                cells[j].1.y
            );
        }
    }
}

#[cucumber::then(regex = r"^everything is on the canvas$")]
async fn everything_on_canvas(world: &mut BusbarWorld) {
    let (_, layout) = layout_of(world);
    for (tag, p) in &layout.places {
        assert!(p.x >= 0.0 && p.y >= 0.0, "`{tag}` is off-canvas at origin");
        assert!(
            p.x + p.w <= layout.width + LAYOUT_EPS,
            "`{tag}` overflows the right edge"
        );
        assert!(
            p.y + p.h <= layout.height + LAYOUT_EPS,
            "`{tag}` overflows the bottom edge"
        );
    }
}

#[cucumber::then(regex = r"^every resolvable edge is routed$")]
async fn every_resolvable_edge_is_routed(world: &mut BusbarWorld) {
    let (ir, layout) = layout_of(world);
    let mut expected: BTreeMap<(String, String), usize> = BTreeMap::new();
    for edge in &ir.edges {
        if let (Some((a, _, _)), Some((b, _, _))) = (
            ir.resolve_endpoint(&edge.from),
            ir.resolve_endpoint(&edge.to),
        ) {
            *expected.entry((a, b)).or_insert(0) += 1;
        }
    }
    for ((a, b), n) in expected {
        let got = layout
            .routes
            .iter()
            .filter(|r| r.from_tag == a && r.to_tag == b)
            .count();
        assert_eq!(
            got, n,
            "edge `{a}` -> `{b}` routed {got}x, expected {n}x — a dropped route is a dropped wire"
        );
    }
}

#[cucumber::then(regex = r"^all routes are orthogonal$")]
async fn all_routes_are_orthogonal(world: &mut BusbarWorld) {
    let (_, layout) = layout_of(world);
    for route in &layout.routes {
        for (a, b) in route.points.iter().zip(route.points.iter().skip(1)) {
            assert!(
                (a.0 - b.0).abs() < LAYOUT_EPS || (a.1 - b.1).abs() < LAYOUT_EPS,
                "route `{}` -> `{}` has a diagonal segment",
                route.from_tag,
                route.to_tag
            );
        }
    }
}

#[cucumber::then(regex = r"^protection sits above the loads of its circuit$")]
async fn protection_above_loads(world: &mut BusbarWorld) {
    let (ir, layout) = layout_of(world);
    for (ctag, circuit) in &ir.circuits {
        let Some(prot) = &circuit.protection else {
            continue;
        };
        let pp = layout
            .places
            .get(&prot.tag)
            .unwrap_or_else(|| panic!("{ctag}: protection `{}` not placed", prot.tag));
        for (load, ..) in &circuit.loads {
            let lp = layout
                .places
                .get(load.as_str())
                .unwrap_or_else(|| panic!("{ctag}: load `{load}` not placed"));
            assert!(
                pp.y < lp.y,
                "{ctag}: protection `{}` sits below load `{load}`",
                prot.tag
            );
        }
    }
}

/// Column key for cell counting: x is on a float grid, so bucket at 0.1
/// granularity before grouping.
fn column_key(x: f64) -> i64 {
    (x * 10.0).round() as i64
}

#[cucumber::then(regex = r"^no feeder column holds more than 4 cells$")]
async fn no_oversized_feeder_columns(world: &mut BusbarWorld) {
    let (ir, layout) = layout_of(world);
    for (ctag, circuit) in &ir.circuits {
        let mut columns: BTreeMap<i64, usize> = BTreeMap::new();
        let mut count = |tag: &str| {
            let p = layout
                .places
                .get(tag)
                .unwrap_or_else(|| panic!("{ctag}: cell `{tag}` not placed"));
            *columns.entry(column_key(p.x)).or_insert(0) += 1;
        };
        if let Some(prot) = &circuit.protection {
            count(&prot.tag);
        }
        if let Some(ctl) = &circuit.controller {
            count(&ctl.tag);
        }
        for (load, ..) in &circuit.loads {
            count(load.as_str());
        }
        for (x, n) in columns {
            assert!(
                n <= busbar_layout::MAX_COL_CELLS,
                "{ctag}: column at x={} holds {n} cells (max {})",
                x as f64 / 10.0,
                busbar_layout::MAX_COL_CELLS
            );
        }
    }
}

#[cucumber::then(regex = r#"^the loads of circuit "([^"]+)" occupy more than one feeder column$"#)]
async fn loads_wrap_into_sub_columns(world: &mut BusbarWorld, ctag: String) {
    let (ir, layout) = layout_of(world);
    let Some(circuit) = ir.circuits.get(&ctag) else {
        panic!("circuit `{ctag}` unknown");
    };
    let mut xs: Vec<i64> = Vec::new();
    for (load, ..) in &circuit.loads {
        let p = layout
            .places
            .get(load.as_str())
            .unwrap_or_else(|| panic!("{ctag}: load `{load}` not placed"));
        let key = column_key(p.x);
        if !xs.contains(&key) {
            xs.push(key);
        }
    }
    assert!(
        xs.len() > 1,
        "loads of {ctag} all landed in one column — the ragged-cascade regression"
    );
}

#[cucumber::then(regex = r"^fed boards hang below the board that feeds them$")]
async fn fed_boards_hang_below(world: &mut BusbarWorld) {
    let (ir, layout) = layout_of(world);
    for (btag, board) in &ir.boards {
        for incomer in &board.incomers {
            let Some((feeder, _)) = incomer.rsplit_once('.') else {
                continue;
            };
            let Some(parent) = feeder.split('.').next() else {
                continue;
            };
            if parent == btag.as_str() || !ir.boards.contains_key(parent) {
                continue;
            }
            let child = layout
                .places
                .get(btag)
                .unwrap_or_else(|| panic!("board `{btag}` not placed"));
            let parent_place = layout
                .places
                .get(parent)
                .unwrap_or_else(|| panic!("board `{parent}` not placed"));
            assert!(
                child.y >= parent_place.y + parent_place.h - LAYOUT_EPS,
                "`{btag}` hangs above its feeding board `{parent}`"
            );
        }
    }
}

#[cucumber::then(
    regex = r#"^the tie "([^"]+)" sits between section "([^"]+)" and section "([^"]+)"$"#
)]
async fn tie_sits_between_sections(
    world: &mut BusbarWorld,
    tie: String,
    upper: String,
    lower: String,
) {
    let (_, layout) = layout_of(world);
    let Some(t) = layout.places.get(&tie) else {
        panic!("tie `{tie}` not placed");
    };
    let (Some(a), Some(b)) = (layout.places.get(&upper), layout.places.get(&lower)) else {
        panic!("sections `{upper}`/`{lower}` not placed");
    };
    assert!(
        t.y > a.y + a.h - LAYOUT_EPS && t.y + t.h < b.y + LAYOUT_EPS,
        "tie `{tie}` must sit between `{upper}` and `{lower}`"
    );
}

#[cucumber::then(regex = r#"^the chain "([^"]+)" stacks above the bar "([^"]+)" in order$"#)]
async fn chain_stacks_above_the_bar(world: &mut BusbarWorld, chain: String, bar: String) {
    let (_, layout) = layout_of(world);
    let Some(bar_place) = layout.places.get(&bar) else {
        panic!("bar `{bar}` not placed");
    };
    let mut prev_y: Option<f64> = None;
    for tag in chain.split(',').map(str::trim) {
        let Some(p) = layout.places.get(tag) else {
            panic!("device `{tag}` not placed");
        };
        assert!(
            p.y + p.h <= bar_place.y + LAYOUT_EPS,
            "`{tag}` sits below the bar `{bar}`"
        );
        if let Some(above_y) = prev_y {
            assert!(
                above_y < p.y,
                "`{tag}` breaks power order — it must stack below the device above it"
            );
        }
        prev_y = Some(p.y);
    }
}

#[cucumber::then(
    regex = r#"^board "([^"]+)" hangs below board "([^"]+)" aligned with feeder "([^"]+)"$"#
)]
async fn board_hangs_below_aligned(
    world: &mut BusbarWorld,
    sub: String,
    parent: String,
    feeder: String,
) {
    let (_, layout) = layout_of(world);
    let Some(s) = layout.places.get(&sub) else {
        panic!("board `{sub}` not placed");
    };
    let Some(p) = layout.places.get(&parent) else {
        panic!("board `{parent}` not placed");
    };
    let Some(f) = layout.places.get(&feeder) else {
        panic!("feeder `{feeder}` not placed");
    };
    assert!(
        s.y >= p.y + p.h - LAYOUT_EPS,
        "`{sub}` must hang below `{parent}`"
    );
    let (sub_cx, _) = s.center();
    let (feeder_cx, _) = f.center();
    assert!(
        (sub_cx - feeder_cx).abs() <= LAYOUT_EPS,
        "`{sub}` must align with its feeder `{feeder}` column"
    );
}

// -- CLI steps (features/cli.feature) -------------------------------------

#[cucumber::given(regex = r#"^a scratch output path "([^"]+)"$"#)]
async fn scratch_output_path(_world: &mut BusbarWorld, path: String) {
    // Workspace-relative, suite-created, parent guaranteed to exist — and
    // cleared first so a stale file can never satisfy an existence assert.
    let full = std::path::PathBuf::from(ROOT).join(&path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("create scratch parent dir");
    }
    let _ = std::fs::remove_file(&full);
}

#[cucumber::given(regex = r#"^a scratch copy of "([^"]+)" at "([^"]+)"$"#)]
async fn scratch_copy(_world: &mut BusbarWorld, from: String, to: String) {
    // Copies a corpus file to a workspace-relative scratch path so
    // in-place commands (fmt) never mutate the corpus itself.
    let src = PathBuf::from(ROOT).join(&from);
    let dst = PathBuf::from(ROOT).join(&to);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).expect("create scratch parent dir");
    }
    std::fs::copy(&src, &dst)
        .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
}

#[cucumber::when(regex = r"^I run `busbar(.*)`$")]
async fn run_cli(world: &mut BusbarWorld, args: String) {
    let argv: Vec<&str> = args.split_whitespace().collect();
    // Prefer the prebuilt binary (CI builds it first) — nested `cargo run`
    // inside `cargo test` contends for the build lock and flakes on
    // Windows. Fall back to cargo only when the binary is missing.
    let bin = std::path::PathBuf::from(ROOT).join("target/debug/busbar");
    if !bin.is_file() {
        let status = std::process::Command::new("cargo")
            .current_dir(ROOT)
            .args(["build", "-q", "-p", "busbar-cli"])
            .status()
            .expect("failed to spawn cargo build");
        assert!(status.success(), "cargo build -p busbar-cli failed");
    }
    let output = std::process::Command::new(&bin)
        .current_dir(ROOT)
        .args(&argv)
        .output()
        .expect("failed to spawn busbar");
    world.cli = Some((
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ));
}

#[cucumber::then(regex = r"^the exit code is (\d+)$")]
async fn exit_code_is(world: &mut BusbarWorld, code: i32) {
    let (actual, _, _) = world.cli.as_ref().expect("`When I run` not executed");
    assert_eq!(*actual, code, "exit code mismatch");
}

#[cucumber::then(regex = r#"^stdout mentions "([^"]+)"$"#)]
async fn stdout_mentions(world: &mut BusbarWorld, needle: String) {
    let (_, stdout, _) = world.cli.as_ref().expect("`When I run` not executed");
    assert!(
        stdout.contains(&needle),
        "stdout {stdout:?} lacks {needle:?}"
    );
}

#[cucumber::then(regex = r#"^stderr mentions "([^"]+)"$"#)]
async fn stderr_mentions(world: &mut BusbarWorld, needle: String) {
    let (_, _, stderr) = world.cli.as_ref().expect("`When I run` not executed");
    assert!(
        stderr.contains(&needle),
        "stderr {stderr:?} lacks {needle:?}"
    );
}

#[cucumber::then(regex = r#"^the file "([^"]+)" exists$"#)]
async fn file_exists(_world: &mut BusbarWorld, path: String) {
    assert!(std::path::Path::new(&path).is_file(), "{path} missing");
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
