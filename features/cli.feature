Feature: CLI behaviour
  Implementation plan §5.7: command surface, exit codes, diagnostics.
  0 = clean, 1 = diagnostics/errors, 2 = usage or IO.

  Scenario: Version reports tool and spec
    When I run `busbar -V`
    Then the exit code is 0
    And stdout mentions "esld/1.0"

  Scenario: No command is a usage error
    When I run `busbar`
    Then the exit code is 2

  Scenario: Unknown option is a usage error
    When I run `busbar check --bogus corpus/valid/minimal.esld`
    Then the exit code is 2

  Scenario: Checking a valid document exits 0
    When I run `busbar check corpus/valid/sample1.esld`
    Then the exit code is 0

  Scenario: Checking an invalid document exits 1 and names the rule
    When I run `busbar check corpus/invalid/r104-dual-fed-load.esld`
    Then the exit code is 1
    And stdout mentions "R-104"

  Scenario: Diagnostics render in compiler convention
    When I run `busbar check corpus/invalid/r110-unlisted-feed.esld`
    Then the exit code is 1
    And stdout mentions "r110-unlisted-feed.esld:26:9:"
    And stdout mentions "= note: add `GRID.out` to the `incomers` of `MAIN`"

  Scenario: Syntax errors report E-PARSE-1 on stdout
    When I run `busbar check corpus/invalid/e-parse-missing-colon.esld`
    Then the exit code is 1
    And stdout mentions "E-PARSE-1"

  Scenario: Warning-only diagnostics exit 0
    When I run `busbar check corpus/invalid/r204-phase-imbalance.esld`
    Then the exit code is 0
    And stdout mentions "R-204"

  Scenario: fmt --check flags unformatted input
    When I run `busbar fmt --check corpus/roundtrip/unformatted.esld`
    Then the exit code is 1

  Scenario: fmt --check accepts canonical input
    When I run `busbar fmt --check corpus/valid/sample1.esld`
    Then the exit code is 0

  Scenario: Rendering writes an SVG
    Given a scratch output path "target/cli-out/minimal.svg"
    When I run `busbar render corpus/valid/minimal.esld -o target/cli-out/minimal.svg`
    Then the exit code is 0
    And the file "target/cli-out/minimal.svg" exists

  Scenario: ANSI registry fails cleanly until M8
    When I run `busbar render corpus/valid/minimal.esld --symbols ansi`
    Then the exit code is 1
    And stderr mentions "M8"

  Scenario: -o with multiple inputs is a usage error
    When I run `busbar render corpus/valid/minimal.esld corpus/valid/sample1.esld -o /tmp/x.svg`
    Then the exit code is 2

  Scenario: Checking a missing file is a usage error
    When I run `busbar check corpus/valid/does-not-exist.esld`
    Then the exit code is 2

  Scenario: Checking several files reports every rule
    When I run `busbar check corpus/invalid/r101-unknown-type.esld corpus/invalid/r102-duplicate-tag.esld`
    Then the exit code is 1
    And stdout mentions "R-101"
    And stdout mentions "R-102"

  Scenario: fmt without files is a usage error
    When I run `busbar fmt`
    Then the exit code is 2

  Scenario: render without files is a usage error
    When I run `busbar render`
    Then the exit code is 2

  Scenario: render --symbols without a value is a usage error
    When I run `busbar render corpus/valid/minimal.esld --symbols`
    Then the exit code is 2

  Scenario: fmt canonicalises a scratch copy in place
    Given a scratch copy of "corpus/roundtrip/unformatted.esld" at "target/cli-out/unformatted.esld"
    When I run `busbar fmt target/cli-out/unformatted.esld`
    Then the exit code is 0
    When I run `busbar fmt --check target/cli-out/unformatted.esld`
    Then the exit code is 0
