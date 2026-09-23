Feature: Parsing
  The ESLD grammar (spec §4 lexical structure + §6 statements): every
  statement kind, quantities, includes, and grammar corners. Fixtures:
  statement-tour (all statements), quantities (unit zoo incl. spaced
  units, µ, %, mm2, m), two-section-tie (bus sections + peer tie),
  board-connects (in-board/in-circuit connects), ess-tour/incomer-tour
  (DC and incomer chains), house (full reference), intermittent-loads
  (§8.8 profiles), incomer-chain/fed-subboards/long-feeder (layout
  fixtures), and the converted seed samples.

  Scenario Outline: Valid documents parse cleanly
    Given the document "<file>"
    When I parse it
    Then it parses without error

    Examples:
      | file |
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

  Scenario: A missing type colon is an E-PARSE-1 diagnostic, not a rule hit
    Given the document "invalid/e-parse-missing-colon.esld"
    When I check it
    Then it reports "E-PARSE-1" with severity error at line 7
    And it reports no other errors

  Scenario: A bad voltsys phase value is an E-IR-1 diagnostic
    Given the document "invalid/e-ir-bad-phases.esld"
    When I check it
    Then it reports "E-IR-1" with severity error at line 5
    And it reports no other errors

