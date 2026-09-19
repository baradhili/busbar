Feature: Protection and rating rules (R-3xx)
  Spec §15, R-300 table: R-301..R-313, with code-profile parameters.
  Lands with with M4 (busbar-check).

  @incomplete
  Scenario Outline: Invalid documents report exactly the expected rule
    Given the document "<file>"
    When I check it
    Then it reports "<code>" with severity <severity> at line <line>
    And it reports no other errors

    Examples:
      | file | code | severity | line |
