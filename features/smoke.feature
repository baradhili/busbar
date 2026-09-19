Feature: BusBar scaffold smoke test
  Walking skeleton: proves the Cucumber harness executes inside the Cargo
  workspace before any ESLD implementation exists. The only active feature
  until implementation milestones land.

  Scenario: The workspace builds and the Cucumber harness runs
    Given the BusBar workspace is scaffolded
    Then this scenario passes
