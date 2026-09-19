Feature: Structural validation rules (R-1xx)
  Spec §15, R-100 table: R-101..R-114.
  Lands with with M3 (busbar-check); they drive corpus/invalid/*.

  # Scenario Outline: Invalid documents report exactly the expected rule
  #   Given the document <file>
  #   When I check it
  #   Then it reports <code> at line <line>
  #   And it reports no other errors
  #
  #   Examples:
  #     | file | code | line |
