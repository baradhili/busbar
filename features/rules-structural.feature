Feature: Structural validation rules (R-1xx)
  Spec §15, R-100 table: R-101..R-114. Fixtures in corpus/invalid/, one
  document per rule. Warnings may accompany the expected diagnostic; the
  "no other errors" step constrains errors only.

  Scenario Outline: Every valid corpus document checks clean
    Given the document "<file>"
    When I check it
    Then there are no error diagnostics

    Examples:
      | file |
      | valid/board-connects.esld |
      | valid/ess-tour.esld |
      | valid/fed-subboards.esld |
      | valid/house.esld |
      | valid/incomer-chain.esld |
      | valid/incomer-tour.esld |
      | valid/intermittent-loads.esld |
      | valid/long-feeder.esld |
      | valid/minimal.esld |
      | valid/quantities.esld |
      | valid/sample1.esld |
      | valid/sample2.esld |
      | valid/spd-pe-wired.esld |
      | valid/statement-tour.esld |
      | valid/two-section-tie.esld |
      | valid/voltsys-blocks.esld |

  Scenario: SPD wired only through PE keeps implicit busbar attachment
    Given the document "valid/spd-pe-wired.esld"
    When I check it
    Then there are no diagnostics

  Scenario Outline: Invalid documents report exactly the expected rule
    Given the document "<file>"
    When I check it
    Then it reports "<code>" with severity <severity> at line <line>
    And it reports no other errors

    Examples:
      | file | code | severity | line |
      | invalid/r101-unknown-type.esld | R-101 | error | 5 |
      | invalid/r102-duplicate-tag.esld | R-102 | error | 7 |
      | invalid/r103-required-port-unconnected.esld | R-103 | error | 5 |
      | invalid/r104-dual-fed-load.esld | R-104 | error | 21 |
      | invalid/r105-unreachable-node.esld | R-105 | warning | 7 |
      | invalid/r106-board-no-incomer.esld | R-106 | error | 5 |
      | invalid/r107-board-no-circuits.esld | R-107 | warning | 7 |
      | invalid/r108-circular-supply.esld | R-108 | error | 24 |
      | invalid/r109-orphan-subboard.esld | R-109 | error | 24 |
      | invalid/r110-unlisted-feed.esld | R-110 | error | 26 |
      | invalid/r111-nonexistent-port.esld | R-111 | error | 38 |
      | invalid/r112-missing-include.esld | R-112 | error | 3 |
      | invalid/r113-no-bus-on-split-board.esld | R-113 | error | 17 |
      | invalid/r114-position-on-non-switch.esld | R-114 | error | 8 |
