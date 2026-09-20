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
      | invalid/r-301-fixture.esld | R-301 | error | 0 |
      | invalid/r-302-fixture.esld | R-302 | error | 0 |
      | invalid/r-303-fixture.esld | R-303 | warning | 0 |
      | invalid/r-304-fixture.esld | R-304 | error | 0 |
      | invalid/r-305-fixture.esld | R-305 | warning | 0 |
      | invalid/r-306-fixture.esld | R-306 | error | 0 |
      | invalid/r-307-fixture.esld | R-307 | error | 0 |
      | invalid/r-308-fixture.esld | R-308 | warning | 0 |
      | invalid/r-309-fixture.esld | R-309 | error | 0 |
      | invalid/r-310-fixture.esld | R-310 | error | 0 |
      | invalid/r-311-fixture.esld | R-311 | warning | 0 |
      | invalid/r-312-fixture.esld | R-312 | error | 0 |
      | invalid/r-313-fixture.esld | R-313 | error | 0 |
