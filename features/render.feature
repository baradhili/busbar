Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes pin
  the IEC primitive registry's output.

  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "c90495efaadc9a949e22918321133706a36f6c948255b11f3d353ff0a0232a36"

  Scenario: Sample 2 golden SVG
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "c83871338fdc9a6c551434e5764914ea8b24a8477f2e32a0525f0136b5106724"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "d0375c5bd0b85fca00647e86a14d5e0d35f25f8c9b28acb6db52edd8754c939d"

  Scenario: Multi-section tie board golden SVG
    Given the document "valid/two-section-tie.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "f28dab879c21218f3ed5142b4ed81ec59dd938c0616b7d393101838e80c01da8"

  Scenario: ESS tour golden SVG
    Given the document "valid/ess-tour.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "946303afdb8844922cf3f12d0e8e174b0d617558ca62d2ce7f50df5e2848ede1"

  Scenario: Simple EV conversion golden SVG
    Given the document "EV/simple.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "0d5946c20b80223b080373b26703046af9079fbf95a3aa6e7cec1f8da2a4ef85"

  Scenario: Complex EV conversion golden SVG
    Given the document "EV/complex_preview.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "a4a7bf3c70ae0e9485433b21d45991248586de552794d84cd241ba069da52f3a"

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
