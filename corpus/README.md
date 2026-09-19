# Conformance corpus

Plain `.esld` fixtures driven by the Gherkin scenarios in `../features/`
(see `Design/esld-implementation.md` §6). Expectations live in the
feature files' `Examples:` tables — expected rule code, line, and severity —
not in sidecar files.

| Directory | Content | First used |
|---|---|---|
| `valid/` | Documents that must parse and check clean | M1/M3 |
| `invalid/` | One minimal document per rule ID where practical | M3–M6 |
| `roundtrip/` | `fmt(parse(x))` idempotence fixtures (comments, unknown props) | M1 |
| `render/` | Documents with golden SVG hashes | M5 |
| `solve/` | Scenario/state/interlock fixtures | M6 |

`valid/sample1.esld` and `valid/sample2.esld` are the converted RSLD seed
examples (`Design/deepseek.md` §16.1/§16.2, conversion per spec Appendix A).

## Fixture conventions

- Invalid fixtures are named `r<code>-<slug>.esld`, one rule per file,
  minimal but otherwise valid — the accompanying `Examples:` row in
  `features/rules-*.feature` asserts the rule code, severity, and 1-based
  anchor line, plus "no other errors" (warnings are permitted).
- Anchor lines are exact: when editing a fixture, re-check the line number
  in the feature file (`grep -n` is your friend).
- Valid fixtures must parse and check clean; they are shared by
  `parsing.feature` and `roundtrip.feature`.
