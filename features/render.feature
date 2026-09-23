Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes pin
  the IEC primitive registry's output.

  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "c6420936e3d5d1be42e883c12c1ab99630c7400f01280480dfe40247ff4be093"

  Scenario: Sample 2 golden SVG
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "c3257422d4e1bb11546dea5a4ad21dc4b8457f153baf4b9c3165dbadb2971f61"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "d78c0bcea7887876232a7dd2783c7a3bc39336d827f4d444b5c1c362b8052c0a"

  Scenario: Multi-section tie board golden SVG
    Given the document "valid/two-section-tie.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "8936eec6c92f3d90a327f02e8068dcf3cbc19d2e3db872813dd5afb92f5b0160"

  Scenario: ESS tour golden SVG
    Given the document "valid/ess-tour.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "5ba51395594e43eb6754c6a34e92027ea90fd539c8fcea58dc7fb78b672922d7"

  Scenario: Rendering is byte-deterministic
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    And I render it again
    Then both rendered outputs are byte-identical

  Scenario Outline: Every valid corpus document renders
    Given the document "<file>"
    When I render it with symbols "iec"
    Then the render succeeds

    Examples:
      | file |
      | valid/board-connects.esld |
      | valid/house.esld |
      | valid/incomer-tour.esld |
      | valid/ess-tour.esld |
      | valid/minimal.esld |
      | valid/quantities.esld |
      | valid/statement-tour.esld |
      | valid/two-section-tie.esld |
      | valid/sample1.esld |
      | valid/sample2.esld |
      | valid/voltsys-blocks.esld |
      | valid/incomer-chain.esld |
      | valid/fed-subboards.esld |
      | valid/long-feeder.esld |

  Scenario: Unknown symbol registries fail cleanly
    Given the document "valid/minimal.esld"
    When I render it with symbols "ansi"
    Then the render fails mentioning "M8"
