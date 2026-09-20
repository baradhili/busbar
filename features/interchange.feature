Feature: Interchange and export
  Spec §17 / implementation plan §5.7: JSON IR, DOT, CSV schedules.
  Lands with M2 (`busbar export`).

  @incomplete
  Scenario Outline: Export formats
    Given the document "valid/sample1.esld"
    When I run `busbar export --format <format>`
    Then the exit code is 0
    And stdout mentions "<marker>"

    Examples:
      | format | marker |
      | ir | "profile" |
      | dot | "digraph" |
      | csv | "board,circuit" |

  @incomplete
  Scenario: Round-trip through the JSON IR
    Given the document "valid/sample1.esld"
    When I export and reload the IR
    Then the reloaded document checks identically
