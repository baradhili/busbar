Feature: Round-trip stability
  Spec §17.1: parse(serialize(parse(text))) == parse(text), with comments
  and unknown properties preserved.

  Scenario Outline: Formatting is idempotent and semantics-preserving
    Given the document "<file>"
    When I format it
    And I format it
    Then both formatted outputs are identical
    And parsing the formatted output yields an AST equal to the original's

    Examples:
      | file |
      | roundtrip/comments-and-unknown-props.esld |
      | roundtrip/unformatted.esld |
      | valid/minimal.esld |
      | valid/board-connects.esld |
      | valid/quantities.esld |
      | valid/statement-tour.esld |
      | valid/two-section-tie.esld |
      | valid/voltsys-blocks.esld |
      | valid/spd-pe-wired.esld |
      | valid/ess-tour.esld |
      | valid/house.esld |
      | valid/incomer-tour.esld |
      | valid/intermittent-loads.esld |
      | valid/incomer-chain.esld |
      | valid/fed-subboards.esld |
      | valid/long-feeder.esld |
      | valid/sample1.esld |
      | valid/sample2.esld |
