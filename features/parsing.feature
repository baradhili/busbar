@phase1
Feature: Parsing
  The ESLD grammar (spec §6): statements, values, quantities, includes.
  Red until busbar-syntax lands (M1); run with BUSBAR_PHASE1=1.

  Scenario Outline: Valid documents parse cleanly
    Given the document "<file>"
    When I parse it
    Then it parses without error

    Examples:
      | file |
      | valid/minimal.esld |
      | valid/voltsys-blocks.esld |
      | valid/sample1.esld |
      | valid/sample2.esld |

  # Scenario: Every fenced esld block in the spec parses
  #   Blocked on tagging the spec's ESLD fences as ```esld so the extractor
  #   can tell them apart from ```ebnf / ```json blocks. Lands with M1.
