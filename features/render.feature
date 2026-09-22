Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes pin
  the IEC primitive registry's output.

  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "8faee7634d21247982d3017f7c41e30e4bcd19f571fc00fed8796ff685848b04"

  Scenario: Sample 2 golden SVG
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "cbfa36d39880e8f31171527cf30935ac8c9477b8f58ee1b392726fbd47e139f8"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "16fe4765b5aff59ceeded0815c3b5e3b4b194d44c994e960d33ed4306a1a17dd"

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
