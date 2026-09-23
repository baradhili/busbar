Feature: Layout invariants
  Spec §16.1 / layout-guidance §6: the drawing is a pure function of the
  IR. Glyphs never overlap and stay on the canvas; every resolvable edge
  is routed as an orthogonal wire (a dropped route is a dropped wire);
  feeder columns stack protection above their loads and wrap at the
  balancing threshold; fed boards hang below the board that feeds them.
  Exact terminal-landing geometry stays in busbar-layout's unit suite.

  Scenario Outline: Valid documents lay out without drafting violations
    Given the document "<file>"
    When I lay it out
    Then no glyphs overlap
    And everything is on the canvas
    And every resolvable edge is routed
    And all routes are orthogonal
    And protection sits above the loads of its circuit
    And no feeder column holds more than 4 cells
    And fed boards hang below the board that feeds them

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

  Scenario: Bus ties render between their sections
    Given the document "valid/two-section-tie.esld"
    When I lay it out
    Then the tie "CB_TIE" sits between section "MSB.A" and section "MSB.B"
    And the tie "CB_TIE2" sits between section "MSB.A" and section "MSB.B"

  Scenario: Incomer chains stack above the bar in power order
    Given the document "valid/incomer-chain.esld"
    When I lay it out
    Then the chain "F1, M1, Q1, OUT" stacks above the bar "MAIN.bus" in order

  Scenario: A fed board hangs below its feeder, aligned with its column
    Given the document "valid/fed-subboards.esld"
    When I lay it out
    Then board "SUB_A" hangs below board "MAIN" aligned with feeder "MAIN.FEED_A"

  Scenario: Long load chains wrap into adjacent sub-columns
    Given the document "valid/long-feeder.esld"
    When I lay it out
    Then the loads of circuit "MAIN.LIGHTS" occupy more than one feeder column
