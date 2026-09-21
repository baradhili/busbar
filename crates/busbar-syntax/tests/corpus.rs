//! Corpus walk: every `.esld` file under the repo corpus must parse, and
//! the formatter must be idempotent on all of them (unit layer under the
//! Cucumber suites; implementation plan §6.3).

use std::path::PathBuf;

fn corpus_files() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let mut files = Vec::new();
    for dir in ["valid", "invalid", "roundtrip", "render", "solve"] {
        let dir = root.join(dir);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "esld") {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn all_corpus_documents_parse() {
    let files = corpus_files();
    assert!(!files.is_empty(), "no corpus files found");
    for path in files {
        // `e-*` fixtures in corpus/invalid are expected E-LEX/E-PARSE
        // errors — asserting they fail to parse is the Cucumber suite's
        // job (features/parsing.feature).
        if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("e-"))
        {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        match busbar_syntax::parse(&src) {
            Ok(_) => {}
            Err(e) => panic!("{}: line {}: {}", path.display(), e.line, e.message),
        }
    }
}

#[test]
fn formatter_is_idempotent_on_corpus() {
    for path in corpus_files() {
        let src = std::fs::read_to_string(&path).unwrap();
        let once = busbar_syntax::fmt::format(&src).expect("lex failure");
        let twice = busbar_syntax::fmt::format(&once).expect("re-lex failure");
        assert_eq!(
            once,
            twice,
            "formatter not idempotent on {}",
            path.display()
        );
    }
}

#[test]
fn formatting_preserves_significant_tokens() {
    for path in corpus_files() {
        let src = std::fs::read_to_string(&path).unwrap();
        let once = busbar_syntax::fmt::format(&src).expect("lex failure");
        let a = busbar_syntax::fmt::significant_tokens(&src).unwrap();
        let b = busbar_syntax::fmt::significant_tokens(&once).unwrap();
        assert_eq!(a, b, "formatting changed tokens of {}", path.display());
    }
}
