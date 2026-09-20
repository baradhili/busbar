Feature: Voltage, phase, frequency, earthing rules (R-2xx)
  Spec §15, R-200 table: R-201..R-210. R-201 fires on nominal-voltage or
  phase-configuration mismatches; R-202 on frequency mismatches alone;
  R-209/R-210 validate intermittent load profiles (§8.8).

  Scenario Outline: Invalid documents report exactly the expected rule
    Given the document "<file>"
    When I check it
    Then it reports "<code>" with severity <severity> at line <line>
    And it reports no other errors

    Examples:
      | file | code | severity | line |
      | invalid/r201-voltage-mismatch.esld | R-201 | error | 9 |
      | invalid/r202-frequency-mismatch.esld | R-202 | error | 9 |
      | invalid/r203-phase-missing-3ph.esld | R-203 | error | 14 |
      | invalid/r204-phase-imbalance.esld | R-204 | warning | 11 |
      | invalid/r205-multiphase-load-on-1ph.esld | R-205 | error | 20 |
      | invalid/r206-neutral-without-neutral.esld | R-206 | error | 17 |
      | invalid/r207-earthing-tie.esld | R-207 | error | 35 |
      | invalid/r208-parallel-transformers-vector.esld | R-208 | warning | 12 |
      | invalid/r208-qualified-endpoints.esld | R-208 | warning | 19 |
      | invalid/r209-bad-duration-unit.esld | R-209 | error | 19 |
      | invalid/r209-on-time-exceeds-period.esld | R-209 | error | 19 |
      | invalid/r210-malformed-window.esld | R-210 | error | 19 |
      | invalid/r210-unknown-season.esld | R-210 | error | 19 |

  Scenario: Intermittent load profiles check clean
    Given the document "valid/intermittent-loads.esld"
    When I check it
    Then there are no error diagnostics
