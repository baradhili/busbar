Feature: Source, paralleling, islanding rules (R-4xx)
  Spec §15, R-400 table: R-401..R-410.
  Lands with with M4 (busbar-check).

  @incomplete
  Scenario Outline: Invalid documents report exactly the expected rule
    Given the document "<file>"
    When I check it
    Then it reports "<code>" with severity <severity> at line <line>
    And it reports no other errors

    Examples:
      | file | code | severity | line |
