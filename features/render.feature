Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes per
  corpus/render/*. Lands with M5 (busbar-render).

  @incomplete
  Scenario: Identical input renders identical SVG
    Given the document "valid/sample1.esld"
    When I render it with symbols "iec"
    Then the SVG hash is "pending-golden-hash"
