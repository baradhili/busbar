@phase1
Feature: Round-trip stability
  Spec §17.1: parse(serialize(parse(text))) == parse(text), with comments
  and unknown properties preserved. Red until busbar fmt lands (M1);
  run with BUSBAR_PHASE1=1.

  Scenario Outline: Formatting is idempotent and semantics-preserving
    Given the document "<file>"
    When I format it
    And I format it
    Then both formatted outputs are identical
    And parsing the formatted output yields an AST equal to the original's

    Examples:
      | file |
      | roundtrip/comments-and-unknown-props.esld |
      | valid/minimal.esld |
      | valid/sample1.esld |
      | valid/sample2.esld |
