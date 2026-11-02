"""Builds the text preview page: template.html plus the font data, into index.html.

Run from anywhere:

    python site/text-preview/build.py          # write index.html
    python site/text-preview/build.py --check  # fail if index.html is out of date

The fonts come from assets/fonts/x11-misc-fixed (public domain BDF files), so the
page shows exactly the glyphs ledkit ships.
"""

import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
FONTS = HERE.parent.parent / "assets" / "fonts" / "x11-misc-fixed"
NAMES = ["4x6", "5x7", "5x8", "6x10", "6x13B", "7x13B", "7x14B", "8x13B", "9x15B", "9x18B", "10x20"]


def read_bdf(path):
    """One BDF font as {w, h, ascent, glyphs: {code: ["0110", ...]}}, ASCII only."""
    lines = path.read_text(encoding="latin-1").splitlines()
    fw = fh = fx = 0
    ascent = 0
    glyphs = {}
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("FONTBOUNDINGBOX"):
            fw, fh, fx, _ = map(int, line.split()[1:])
        elif line.startswith("FONT_ASCENT"):
            ascent = int(line.split()[1])
        elif line.startswith("STARTCHAR"):
            enc = bbx = None
            j = i + 1
            while not lines[j].startswith("BITMAP"):
                if lines[j].startswith("ENCODING"):
                    enc = int(lines[j].split()[1])
                elif lines[j].startswith("BBX"):
                    bbx = list(map(int, lines[j].split()[1:]))
                j += 1
            rows = []
            j += 1
            while not lines[j].startswith("ENDCHAR"):
                rows.append(int(lines[j], 16))
                j += 1
            if enc is not None and 32 <= enc <= 126:
                bw, bh, bx, by = bbx
                nbytes = (bw + 7) // 8
                grid = [[0] * fw for _ in range(fh)]
                top = ascent - (by + bh)  # cell row where the glyph's bitmap starts
                for r, bits in enumerate(rows):
                    for c in range(bw):
                        if bits >> (nbytes * 8 - 1 - c) & 1:
                            x, y = bx - fx + c, top + r
                            if 0 <= x < fw and 0 <= y < fh:
                                grid[y][x] = 1
                glyphs[enc] = ["".join(map(str, row)) for row in grid]
            i = j
        i += 1
    return {"w": fw, "h": fh, "ascent": ascent, "glyphs": glyphs}


def build():
    fonts = {name: read_bdf(FONTS / f"{name}.bdf") for name in NAMES}
    data = "const BDF_FONTS = " + json.dumps(fonts, separators=(",", ":")) + ";"
    template = (HERE / "template.html").read_text(encoding="utf-8")
    return template.replace("/*FONTS*/", data)


def main():
    page = build()
    out = HERE / "index.html"
    if "--check" in sys.argv:
        if not out.exists() or out.read_text(encoding="utf-8") != page:
            sys.exit("site/text-preview/index.html is out of date: run python site/text-preview/build.py")
        print("index.html is up to date")
    else:
        out.write_text(page, encoding="utf-8", newline="\n")
        print(f"wrote {out}")


if __name__ == "__main__":
    main()
