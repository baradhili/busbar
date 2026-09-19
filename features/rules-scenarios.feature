Feature: Scenario rules (R-5xx)
  Spec §15, R-500 table: R-501..R-503.
  Lands with with M6 (busbar-solve).

  @incomplete
  Scenario Outline: Invalid documents report exactly the expected rule
    Given the document "<file>"
    When I check it
    Then it reports "<code>" with severity <severity> at line <line>
    And it reports no other errors

    Examples:
      | file | code | severity | line |
