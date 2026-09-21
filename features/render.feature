Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes pin
  the IEC primitive registry's output.

  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "bcc6c3defce9cfe527ceefeda13e2172394cd079543ce22bffecac525273e64c"

  Scenario: Sample 2 golden SVG
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "f836d85623b15f6e579d776ea1f8ff2ba15f21d7967693dc7920b2021645eca7"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "b2bca9e121b8e4de10324c2e9fbd6c468fb2b8f68715c9ee7f40a4df71e731b1"

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
