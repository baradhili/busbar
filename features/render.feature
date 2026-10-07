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
    Then the SVG hash is "2bf49b948ea0fc1dd3b05bfe3b38d9693f10abf145643432231e5305039e98d9"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "1094f9499db50fc9105c03370bd03f92efa31303e18857d8bae9cbb9328a6f25"

  Scenario: Multi-section tie board golden SVG
    Given the document "valid/two-section-tie.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "cc35664d3440be56244829ba8716298a54af821660de913da29d294e136e4d51"

  Scenario: ESS tour golden SVG
    Given the document "valid/ess-tour.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "5bbeb94158fae30ed51aaf72dea4055ab3dc53611645df8aa234a3f46da203cb"

  Scenario: Simple EV conversion golden SVG
    Given the document "EV/simple.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "b3ea315acef6e9491c06d63f8c7691cfe18dab4b8fff907352034111a6a6797a"

  Scenario: Complex EV conversion golden SVG
    Given the document "EV/complex_preview.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "b73c2e7cacb3bd775a7ee7becec8dffe4d567341ef951a159d0af56a83e285f9"

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
      | EV/simple.esld |
      | EV/complex_preview.esld |

  Scenario: Unknown symbol registries fail cleanly
    Given the document "valid/minimal.esld"
    When I render it with symbols "ansi"
    Then the render fails mentioning "M8"
