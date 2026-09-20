# Extracted symbol reference library

Machine-extracted from the user-supplied sources in `Design/symbols/`
(**not committed** — gitignored; IEC/ISO/BS and third-party copyrighted
material that must not be redistributed). These extracts are an internal
visual reference for drawing and improving BusBar's own IEC-style
primitive glyphs (`busbar-render`). **The renderer never embeds any of
this artwork** — per the licensing stance in `busbar-render/src/lib.rs`
and `Design/layout-guidance.md` §4, symbols are drawn from vector
primitives. If this repository is ever published, this directory needs a
full licensing review first.

## Contents

| Path | What | How extracted |
|---|---|---|
| `index.csv` (top) | 4101 equipment symbols (ISO 7000 / IEC 60417 overview poster, 2018-03-02 edition) with registry numbers + titles | poster SVG's embedded base64 PNGs paired with their `<text>` tspans by matching `image`/`text` element ids |
| `poster/` (gitignored bulk) | the PNGs above | — |
| `iec60617/` | 524 schematic symbol cells from the IEC 60617 table PDF: `png/` (300 dpi crops) + `svg/` (potrace traces) + `index.csv` | description-column rows located by text geometry (pdfminer), symbol-column cells cropped from 300 dpi page renders, traced with potrace (pure-python `potracer`) |
| `elec-mate/` | `page-1..5.svg` — full-page true-vector exports of the UK Elec-Mate chart (IEC 60617 / BS 7671 installation symbols, 114 symbols) | `pdftocairo -svg` (the chart is native vector) |
| `visio/` | 175 unique master shapes as true vector SVGs + `index.csv` | `tools/extract-symbols/vssx_to_svg.py` converts Visio 2013+ stencil geometry (MoveTo/LineTo/ArcTo/Ellipse/InfiniteLine/RelCubBezTo rows, group transforms, flips) to SVG paths; sha256-deduped across stencils |

## Sources (gitignored)

- `IEC-ISO Database on Graphical Symbols for Use on Equipment.{pdf,svg}` — ISO 7000/IEC 60417 equipment-marking symbols (mostly NOT schematic drafting symbols; the electrical subset is small)
- `pdfcoffee.com_iec-60617-symbols-pdf-free.pdf` — 53-page IEC 60617 symbol table (the primary drafting reference here)
- `elec-mate-electrical-symbols-chart.pdf` — UK installation-symbols chart (closest to BusBar's residential/ LV domain)
- `Electrical.vssx` — 10 LV solar/battery-domain masters (breaker, RCD, timer switch, watt-hour meter, inverter-charger, isolators, grid, ATS ×2)
- `visio-electrical-master/` — GOST + RU/EN/NL stencil collection (transformers, switches, outlets, protection, appliances)

## Pending / limitations

- **91 binary `.vss` GOST stencils** (`GOSTs_templates/`, `library/`):
  the Visio 2003-2010 OLE binary format. No local tool parses it
  (LibreOffice+libvisio or Visio itself required). The 18 XML `.vssx`
  stencils in the same collection are fully converted.
- Poster title coverage is 4101/4501 images (401 images lack a matching
  text block); unmatched images were dropped rather than mislabeled.
- 60617 cell crops pair symbol art with descriptions by row geometry; a
  handful of rows on 8 pages had image/row count drift and are
  flagged in the extraction history (cells are never mislabeled —
  unmatchable rows are skipped).
- elec-mate symbols are page-level; per-symbol splitting of its vector
  paths is future work if needed.

## Re-generating

Extraction is one-shot and documented above; the reusable piece is the
Visio converter: `tools/extract-symbols/vssx_to_svg.py STENCIL.vssx OUTDIR`.
It needs only the stdlib (+ ElementTree); the 60617 tracing used a venv
with `potracer`, `pillow`, `pdfminer.six`.
