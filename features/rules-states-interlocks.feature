Feature: State and interlock rules (R-6xx)
  Spec §15, R-600 table: R-601..R-604.
  Lands with with M6 (busbar-solve).

  @incomplete
  Scenario Outline: Invalid documents report exactly the expected rule
    Given the document "<file>"
    When I check it
    Then it reports "<code>" with severity <severity> at line <line>
    And it reports no other errors

    Examples:
      | file | code | severity | line |
