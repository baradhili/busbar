Feature: Round-trip stability
  Spec §17.1: parse(serialize(parse(text))) == parse(text), with comments
  and unknown properties preserved. Scenarios land with M1 (busbar fmt).

  # Scenario: Formatting is idempotent and semantics-preserving
  #   Given the document <file>
  #   When I format it
  #   And I format the result again
  #   Then both formatted outputs are identical
  #   And parsing the formatted output yields an AST equal to the original's
