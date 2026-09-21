Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes pin
  the IEC primitive registry's output.

  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "0e7da52a657a090287b38db425b63c3a572211272de710b9a5ccd307cd08ba88"

  Scenario: Sample 2 golden SVG
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "0186819d115aae85e9055d0b64ff8f4c070480b685067fee64ed22b914aaf78b"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "4c7edac8f9d88684f899c75c9b59986a99e54eda66fddc425d916b13c6b1ed63"

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
      | valid/minimal.esld |
      | valid/quantities.esld |
      | valid/statement-tour.esld |
      | valid/two-section-tie.esld |
      | valid/sample1.esld |
      | valid/sample2.esld |
      | valid/voltsys-blocks.esld |

  Scenario: Unknown symbol registries fail cleanly
    Given the document "valid/minimal.esld"
    When I render it with symbols "ansi"
    Then the render fails mentioning "M8"
