Feature: WASM target
  Implementation plan §5.8: @busbar/core runs the same checks and
  renders in browser/Node. Lands with M7.

  @incomplete
  @wasm
  Scenario: The WASM build renders the same SVG hash
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec" on the WASM build
    Then the SVG hash is "dbec09bab30d08d15b8d46c76b1e71ec8fb5bb128580b2cf6da1b5b163fd8af4"

  @incomplete
  Scenario: The WASM bundle stays within budget
    When I measure the @busbar/core bundle
    Then it is at most 1.5MB gzipped
