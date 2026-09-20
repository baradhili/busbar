#!/usr/bin/env python3
"""Convert Visio 2013+ (VSSX) stencil masters to standalone SVGs.

Handles the subset of Visio geometry the Electrical.vssx stencil uses:
grouped sub-shapes with pin transforms, MoveTo/LineTo/ArcTo/Ellipse/
InfiniteLine rows, Rel* fractional rows, RelCubBezTo, and the
line/fill pattern cells. Output is stroke-black, fill-none line art.
"""
import re, sys, zipfile, math
import xml.etree.ElementTree as ET

NS = {'v': 'http://schemas.microsoft.com/office/visio/2012/main'}


def q(tag):
    return f'{{{NS["v"]}}}{tag}'


def cells(el):
    return {c.get('N'): c for c in el.findall('v:Cell', NS)}


def v(el, name, default=0.0):
    c = cells(el).get(name)
    if c is None:
        return default
    try:
        return float(c.get('V'))
    except (TypeError, ValueError):
        return default


def color(val, fallback='#000000'):
    s = str(val).strip()
    if s.startswith('#'):
        return s
    # Numeric cells arrive as floats (1.0 == palette index 1): normalize
    # integral floats to their int form before the palette lookup.
    try:
        f = float(s)
        if f == int(f):
            s = str(int(f))
    except ValueError:
        pass
    # Visio palette: 0 black, 1 white; others unused in this stencil
    return {'0': '#000000', '1': '#ffffff'}.get(s, fallback)


class Shape:
    def __init__(self, el):
        self.el = el
        self.name = el.get('Name') or el.get('NameU') or el.get('ID')
        self.pin = (v(el, 'PinX'), v(el, 'PinY'))
        self.locpin = (v(el, 'LocPinX'), v(el, 'LocPinY'))
        self.size = (max(v(el, 'Width'), 1e-9), max(v(el, 'Height'), 1e-9))
        self.angle = v(el, 'Angle')
        self.flipx = v(el, 'FlipX') != 0
        self.flipy = v(el, 'FlipY') != 0
        self.linepat = int(v(el, 'LinePattern', 1))
        self.fillpat = int(v(el, 'FillPattern', 1))
        self.linecolor = color(v(el, 'LineColor', 0))
        self.fillcolor = color(v(el, 'FillForegnd', 1))
        self.children = [Shape(c) for c in el.findall('v:Shapes/v:Shape', NS)]

    def to_parent(self, x, y):
        """Local shape coords -> parent coords (Visio 2-D transform)."""
        w, h = self.size
        if self.flipx:
            x = w - x
        if self.flipy:
            y = h - y
        px, py = self.pin
        lx, ly = self.locpin
        x, y = x - lx, y - ly
        if self.angle:
            c, s = math.cos(self.angle), math.sin(self.angle)
            x, y = x * c - y * s, x * s + y * c
        return x + px, y + py

    def geometry(self):
        """Yield (svg-path-d, stroke, dashed, filled) in local coords."""
        for sec in self.el.findall("v:Section[@N='Geometry']", NS):
            rows = []
            for row in sec.findall('v:Row', NS):
                t = row.get('T') or 'MoveTo'
                rows.append((t, row))
            if not rows:
                continue
            d = []
            cx = cy = 0.0
            for t, row in rows:
                X = v(row, 'X', cx)
                Y = v(row, 'Y', cy)
                w, h = self.size
                if t == 'RelMoveTo':
                    X, Y = X * w, Y * h
                    d.append(f'M {X:.4f} {Y:.4f}')
                elif t == 'MoveTo':
                    d.append(f'M {X:.4f} {Y:.4f}')
                elif t == 'RelLineTo':
                    X, Y = X * w, Y * h
                    d.append(f'L {X:.4f} {Y:.4f}')
                elif t == 'LineTo':
                    d.append(f'L {X:.4f} {Y:.4f}')
                elif t == 'RelCubBezTo':
                    A, B = v(row, 'A', cx) * w, v(row, 'B', cy) * h
                    C, D = v(row, 'C', cx) * w, v(row, 'D', cy) * h
                    d.append(
                        f'C {A:.4f} {B:.4f} {C:.4f} {D:.4f} '
                        f'{X * w:.4f} {Y * h:.4f}'
                    )
                elif t == 'ArcTo':
                    a = v(row, 'A')
                    dx, dy = X - cx, Y - cy
                    chord = math.hypot(dx, dy)
                    if abs(a) < 1e-9 or chord < 1e-9:
                        d.append(f'L {X:.4f} {Y:.4f}')
                    else:
                        r = (chord * chord / 4 + a * a) / (2 * abs(a))
                        sweep = 1 if a > 0 else 0
                        large = 0  # Visio ArcTo bulge is always minor arcs
                        d.append(f'A {r:.4f} {r:.4f} 0 {large} {sweep} {X:.4f} {Y:.4f}')
                elif t == 'Ellipse':
                    # X,Y center; A,B x-radius vec; C,D y-radius vec
                    ax, ay = v(row, 'A', X + 1) - X, v(row, 'B', Y) - Y
                    rx = math.hypot(ax, ay) if (ax or ay) else 1.0
                    bx, by = v(row, 'C', X) - X, v(row, 'D', Y + 1) - Y
                    ry = math.hypot(bx, by) if (bx or by) else 1.0
                    d.append(
                        f'M {X - rx:.4f} {Y:.4f} '
                        f'a {rx:.4f} {ry:.4f} 0 1 0 {2 * rx:.4f} 0 '
                        f'a {rx:.4f} {ry:.4f} 0 1 0 {-2 * rx:.4f} 0 Z'
                    )
                elif t == 'InfiniteLine':
                    X1, Y1 = v(row, 'X1', X + 1), v(row, 'Y1', Y)
                    L = 10.0
                    n = math.hypot(X1 - X, Y1 - Y) or 1.0
                    ux, uy = (X1 - X) / n, (Y1 - Y) / n
                    d.append(f'M {X - ux * L:.4f} {Y - uy * L:.4f}')
                    d.append(f'L {X + ux * L:.4f} {Y + uy * L:.4f}')
                elif t == 'Connection' or t == 'ConnectionABCD':
                    continue
                cx, cy = X, Y
            if not d:
                continue
            filled = self.fillpat != 0
            yield (
                ' '.join(d),
                self.linecolor,
                self.linepat == 2,
                filled,
                self.fillcolor,
            )


def convert(vssx_path, outdir):
    zf = zipfile.ZipFile(vssx_path)
    import os
    os.makedirs(outdir, exist_ok=True)
    index = []
    # Master name/dims come from masters.xml; the i-th Master (document
    # order) pairs with the i-th masterN.xml (1-based ordinal).
    mroot = ET.fromstring(zf.read('visio/masters/masters.xml'))
    seen_slugs = {}
    for i, m in enumerate(mroot.findall('v:Master', NS), start=1):
        mname = m.get('Name') or m.get('NameU') or f'master{i}'
        sheet = m.find('v:PageSheet', NS)
        pw = v(sheet, 'PageWidth', 1.0) if sheet is not None else 1.0
        ph = v(sheet, 'PageHeight', 1.0) if sheet is not None else 1.0
        try:
            root = ET.fromstring(zf.read(f'visio/masters/master{i}.xml'))
        except KeyError:
            continue
        shapes = root.findall('v:Shapes/v:Shape', NS)
        if not shapes:
            continue
        top = []
        for sel in shapes:
            emit_toplevel(Shape(sel), pw, ph, top)
        if not top:
            continue
        slug = re.sub(r'[^a-z0-9]+', '-', mname.lower()).strip('-')[:70] or f'master{i}'
        n = seen_slugs.get(slug, 0) + 1
        seen_slugs[slug] = n
        if n > 1:
            slug = f'{slug}-{n}'
        Wr, Hr = round(pw, 4), round(ph, 4)
        svg = (
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {Wr} {Hr}" '
            f'width="{Wr * 100:.0f}" height="{Hr * 100:.0f}">\n'
            + '\n'.join(top)
            + '\n</svg>\n'
        )
        fn = f'{slug}.svg'
        with open(f'{outdir}/{fn}', 'w') as f:
            f.write(svg)
        index.append((mname, fn))
    print(f'  {len(index)} masters from {os.path.basename(vssx_path)}')
    return index


def transform_path(d, xf):
    """Rewrite a path's absolute coordinates through xf.

    Command-aware: only M, L, and C argument pairs and an absolute A's
    final endpoint are coordinates. Arc radii, x-rotation, and the
    large-arc/sweep flags keep their original text (flags must stay
    `0`/`1`), and relative (lowercase) commands — the ellipse arcs —
    pass through with their deltas untouched.
    """
    parts = re.findall(r'[A-Za-z]|-?[\d.]+', d)
    out = []
    i = 0
    while i < len(parts):
        cmd = parts[i]
        i += 1
        if not cmd.isalpha():
            out.append(cmd)  # defensive: stray token
            continue
        args = []
        while i < len(parts) and not parts[i].isalpha():
            args.append(parts[i])
            i += 1
        if cmd in ('M', 'L', 'C') and len(args) >= 2:
            for j in range(0, len(args) - 1, 2):
                x, y = xf(float(args[j]), float(args[j + 1]))
                args[j], args[j + 1] = f'{x:.4f}', f'{y:.4f}'
        elif cmd == 'A' and len(args) >= 7:
            x, y = xf(float(args[5]), float(args[6]))
            args[5], args[6] = f'{x:.4f}', f'{y:.4f}'
        out.append(cmd + ((' ' + ' '.join(args)) if args else ''))
    return ''.join(out)


def emit_path(out, d, xf, stroke, dashed, filled, fill):
    dash = ' stroke-dasharray="0.02 0.015"' if dashed else ''
    fillattr = f' fill="{fill}"' if filled else ' fill="none"'
    out.append(
        f'<path d="{transform_path(d, xf)}" stroke="{stroke}" stroke-width="0.01"'
        f'{fillattr}{dash}/>'
    )


def emit_toplevel(shape, pw, ph, out):
    """Emit paths with local->page transform, recursing groups."""
    def page(x, y):
        px, py = shape.to_parent(x, y)
        return px, ph - py  # Visio y-up -> SVG y-down
    for d, stroke, dashed, filled, fill in shape.geometry():
        emit_path(out, d, page, stroke, dashed, filled, fill)
    for ch in shape.children:
        emit_child(ch, shape, ph, out)


def emit_child(child, parent, ph, out):
    def page(x, y):
        px, py = child.to_parent(x, y)
        px, py = parent.to_parent(px, py)
        return px, ph - py

    for d, stroke, dashed, filled, fill in child.geometry():
        emit_path(out, d, page, stroke, dashed, filled, fill)
    for ch in child.children:
        emit_child(ch, child, ph, out)


if __name__ == '__main__':
    idx = convert(sys.argv[1], sys.argv[2])
    print(f'{len(idx)} masters converted')
