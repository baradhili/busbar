Feature: Parsing
  The ESLD grammar (spec §6): statements, values, quantities, includes.

  Scenario Outline: Valid documents parse cleanly
    Given the document "<file>"
    When I parse it
    Then it parses without error

    Examples:
      | file |
      | valid/minimal.esld |
      | valid/board-connects.esld |
      | valid/voltsys-blocks.esld |
      | valid/sample1.esld |
      | valid/sample2.esld |

