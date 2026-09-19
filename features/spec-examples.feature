Feature: Spec examples are valid
  The worked examples in Design/esld-spec.md §18 must check clean once
  busbar-check lands; fenced blocks elsewhere must at least parse.
  Lands with M1/M3 (tools/spec-example extractor, blocked on tagging
  the spec's ESLD fences as ```esld).

  @incomplete
  Scenario: Every fenced esld block in the spec at least parses
    Given the spec "Design/esld-spec.md"
    When I extract its fenced example blocks
    Then each parses without error
    And the worked examples (§18) check clean
