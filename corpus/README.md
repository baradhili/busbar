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
