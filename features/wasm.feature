Feature: WASM target
  Implementation plan §5.8: @busbar/core runs the same checks and
  renders in browser/Node. Lands with M7.

  @incomplete
  @wasm
  Scenario: The WASM build renders the same SVG hash
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec" on the WASM build
    Then the SVG hash is "1e0c771a74fde8a63faa0d3f973891a7ebcd914a928fab8cb78a683f88f25383"

  @incomplete
  Scenario: The WASM bundle stays within budget
    When I measure the @busbar/core bundle
    Then it is at most 1.5MB gzipped
