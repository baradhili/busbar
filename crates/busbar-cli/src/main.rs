//! `busbar` — native CLI for the ESLD toolchain.
//!
//! Command surface is specified in `Design/esld-implementation.md` §5.7.
//! Scaffold: no commands implemented yet; exit code 2 (usage) until M1.

fn main() {
    eprintln!(
        "busbar {} — scaffold, no commands implemented yet",
        env!("CARGO_PKG_VERSION")
    );
    std::process::exit(2);
}
