Feature: WASM target
  Implementation plan §5.8: @busbar/core runs the same checks and
  renders in browser/Node. Lands with M7.

  @incomplete
  @wasm
  Scenario: The WASM build renders the same SVG hash
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec" on the WASM build
    Then the SVG hash is "225d2b11b304d291807618a79d2df384551f5867615232fe184e34c9e3dc5deb"

  @incomplete
  Scenario: The WASM bundle stays within budget
    When I measure the @busbar/core bundle
    Then it is at most 1.5MB gzipped
