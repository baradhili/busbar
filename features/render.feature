Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes pin
  the IEC primitive registry's output.

  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "ebce19c8b942e8a8c3c1ddb670b3e7fbc0864cd6051ac1bdc4c0c55294485faf"

  Scenario: Sample 2 golden SVG
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "75ba88593ff509216540803e5a195cd6d8e7aaa569a27e18ee3fb783091beafb"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "6279a5e68842e296ab9adc0e0cb48cb323c3d0db0798a47e2a2fb374b1f4fe42"

  Scenario: Multi-section tie board golden SVG
    Given the document "valid/two-section-tie.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "4087fc42593d80baca4b331983f91aa8a0cc1eb8e47ed85aeea5eb56d936d8c7"

  Scenario: ESS tour golden SVG
    Given the document "valid/ess-tour.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "ac05c9c40981b20a34205a465312c8c1b13f8101425c6cbc52e961e9fd2ad8b5"

  Scenario: Simple EV conversion golden SVG
    Given the document "EV/simple.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "61393cb4209b400f2c3ca00e7b5dc5bf4704664d466f3bb0a75eaf0b9a57e0ff"

  Scenario: Complex EV conversion golden SVG
    Given the document "EV/complex_preview.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "1fd34e0185a225bb2950b764d2a7edcf95ccf50d142c32b46ee20052515ea406"

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
