Feature: Rendering determinism
  Spec §16.2: identical input renders bit-identical SVG; golden hashes per
  corpus/render/*. Scenarios land with M5 (busbar-render).

  # Scenario: Identical input renders identical SVG
  #   Given the document <file>
  #   When I render it with symbols "iec"
  #   Then the SVG hash is <hash>
