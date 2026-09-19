Feature: Parsing
  The ESLD grammar (spec §6): statements, values, quantities, includes.
  Lands with with M1 (busbar-syntax).

  # Scenario: Every fenced esld block in the spec parses
  #   Given the spec "Design/esld-spec.md"
  #   When I extract its fenced example blocks
  #   Then each parses without error
