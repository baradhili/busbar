Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes pin
  the IEC primitive registry's output.

  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "b3e267f89bada248e369921c13eaa2b2c3111a992ca6335bd95a73076a25dd69"

  Scenario: Sample 2 golden SVG
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "b2bdf57a1844ec08202f1d62978ff06f6e79bc285445ce31ae34355e0ca19b39"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "e85c778e4225e78bab4f2c0ae3e307bcb9fbeaa3ad06f6bdc282753630cc216d"

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

  Scenario: Unknown symbol registries fail cleanly
    Given the document "valid/minimal.esld"
    When I render it with symbols "ansi"
    Then the render fails mentioning "M8"
