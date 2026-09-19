Feature: Solver scenarios
  Spec §10.7, §13, §14: states, interlocks, expectations.
  Lands with M6 (busbar-solve).

  @incomplete
  Scenario Outline: Scenario expectations from the spec examples
    Given the document "<file>"
    When I solve scenario "<scenario>"
    Then all expectations hold

    Examples:
      | file | scenario |
      | valid/sample2.esld | Grid loss at night, battery 90% |
