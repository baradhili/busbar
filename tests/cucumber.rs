//! Cucumber acceptance harness — run with `cargo test --test cucumber`.
//!
//! Steps are intentional no-op scaffolds so the suite is green before any
//! ESLD implementation exists. Real step definitions land with the
//! implementation milestones (`Design/esld-implementation.md` §6, §10);
//! scenarios for unimplemented areas stay commented out in `features/`.

use cucumber::World;

/// State carried between steps: source text → parsed document →
/// diagnostics / render output / solve result.
#[derive(Debug, Default, World)]
pub struct BusbarWorld;

#[cucumber::given(regex = r"^the BusBar workspace is scaffolded$")]
async fn workspace_scaffolded(_world: &mut BusbarWorld) {}

#[cucumber::then(regex = r"^this scenario passes$")]
async fn scenario_passes(_world: &mut BusbarWorld) {}

#[tokio::main]
async fn main() {
    BusbarWorld::run("features").await;
}
