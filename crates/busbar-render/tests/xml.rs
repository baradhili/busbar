//! Golden hashes pin byte-determinism but say nothing about XML
//! validity — an unbalanced group or an escaped quote renders "fine" to
//! a hash and breaks every viewer (the INVERTER `</g>` and wrapper
//! `\"` regressions). This suite parses every corpus render with a real
//! XML parser.

use std::path::PathBuf;

fn corpus() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/valid");
    let mut files: Vec<PathBuf> = std::fs::read_dir(root)
        .expect("corpus/valid")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "esld"))
        .collect();
    files.sort();
    files
}

#[test]
fn renders_are_well_formed_xml() {
    for path in corpus() {
        let src = std::fs::read_to_string(&path).unwrap();
        let svg = busbar_render::render_str(&src, "iec")
            .unwrap_or_else(|e| panic!("{}: render failed: {e}", path.display()));
        let mut reader = quick_xml::Reader::from_str(&svg);
        reader.config_mut().allow_unmatched_ends = false;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => panic!("{}: not well-formed XML: {e}", path.display()),
            }
        }
    }
}
