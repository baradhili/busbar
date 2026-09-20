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
      | invalid/r-401-fixture.esld | R-401 | error | 0 |
      | invalid/r-402-fixture.esld | R-402 | error | 0 |
      | invalid/r-403-fixture.esld | R-403 | error | 0 |
      | invalid/r-404-fixture.esld | R-404 | error | 0 |
      | invalid/r-405-fixture.esld | R-405 | error | 0 |
      | invalid/r-406-fixture.esld | R-406 | warning | 0 |
      | invalid/r-407-fixture.esld | R-407 | error | 0 |
      | invalid/r-408-fixture.esld | R-408 | warning | 0 |
      | invalid/r-409-fixture.esld | R-409 | error | 0 |
      | invalid/r-410-fixture.esld | R-410 | error | 0 |
