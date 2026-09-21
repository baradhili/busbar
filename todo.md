# To do list:

- [ ] Place tie devices BETWEEN the sections they join (currently the tie breaker joins the incomer column above the first bar; two-section-tie reference)


- [x] Make wires connect to MCB/RCBO/switch at correct connection point - current both connect to same point

- [x] Incomer feed to busbar should all be above the busbar even if it is not another board (e.g. house.esld grid->fuse->meter->main switch should be a chain above the busbar)

- [x] Fix RCBO symbol - look to ./corpus/render/house.svg (now transcribed from corpus/render/symbols.svg)

- [ ] Review symbol sources: https://github.com/dainyoung-code/sahkocad/tree/main/src , https://github.com/OleJBondahl/Schematika , https://github.com/KarelT1/KiCad-IEC-60617-symbol-library , https://github.com/synergycodes/ng-diagram-single-line-diagram
- [ ] More symbol source candidates: IEC 60617:2024 online database (webstore.iec.ch — the normative set; extract by hand, never commit the copyrighted source), QElectroTech element collections (GPL-2.0, IEC-style, huge coverage), official KiCad symbol library (IEEE/IEC-style dual mode), Wikimedia Commons "IEC 60617" category (mostly public-domain SVG renderings), Dia's electrical shape sheets (GPL), IEC 61082-1 (diagram preparation rules — complements the symbols), AS/NZS 1101 series (AU graphical symbols, matches the AS/NZS 3000 code profile)
  Note: keep the .gitignore stance — copyrighted source files stay out of the repo; only transcribed geometry lands in busbar-render.

- [ ] load symbols overlap wires - or are not actually connected to wires

- [ ] PV inverter doesn't connect to wire - also needs to be direct above their connection to a bus.

- [x] Incomer breakers should be above a bus, not below