Feature: Parsing
  The ESLD grammar (spec §4 lexical structure + §6 statements): every
  statement kind, quantities, includes, and grammar corners. Fixtures:
  statement-tour (all statements), quantities (unit zoo incl. spaced
  units, µ, %, mm2, m), two-section-tie (bus sections + peer tie),
  board-connects (in-board/in-circuit connects).

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
      | valid/sample1.esld |
      | valid/sample2.esld |

