Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes pin
  the IEC primitive registry's output.

  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "96d4934dd6b2f651b745a5ea0759fd928b93bd6590fb8f2457bb5750cf3c0370"

  Scenario: Sample 2 golden SVG
    Given the document "valid/sample2.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "0ef339be08ebe7a8eaeb9b5b5a718c5700a9ebcb5d8a4a6217a389631fb91684"

  Scenario: House reference golden SVG
    Given the document "valid/house.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "cd55470ca87e40e0fd448438b45f9a1b30ca0c4a448e9ad2d90fe546903f0baf"

  Scenario: Multi-section tie board golden SVG
    Given the document "valid/two-section-tie.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "85fe65016c492a84b6ef101d071149012566709ce8033304c88798213064fbec"

  Scenario: ESS tour golden SVG
    Given the document "valid/ess-tour.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "93978c4350f2b38c2b8747ddbc9a2396046b019531fd157b4d210207da10530c"

  Scenario: Simple EV conversion golden SVG
    Given the document "EV/simple.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "8f3c4abecdc90c44bad3f38033f852da3e782271dd196eb989035344c4e47375"

  Scenario: Complex EV conversion golden SVG
    Given the document "EV/complex_preview.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "3b92448e70a7fe11c313e8c200e1b58fb2d872e07b53eeb2943ad96fd4da65ec"

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
