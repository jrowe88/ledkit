"""Bakes the bitmap fonts into coverage tables, shared by ledkit-text and the preview page.

    python tools/fontgen.py          # write the generated files
    python tools/fontgen.py --check  # fail if any generated file is out of date

For each face and each size (1x, 1.25x, 1.5x, 2x) every glyph becomes a table of one byte
per LED: the sum of 16 samples, 0..=16. 1x is the font's own pixels (16 or 0). Bigger sizes
enlarge the glyph cell with Scale2x three times (8x), then take 4x4 samples per LED.
This is the preview's `antialias`, applied to one glyph cell at a time.

Outputs: crates/ledkit-text/src/faces/data/<face>.bin and mod.rs, site/text-preview/faces.json.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
FONTS = ROOT / "assets" / "fonts"
BDF_DIR = FONTS / "x11-misc-fixed"
TINY_FILE = FONTS / "ledkit-tiny-3x5.txt"
DATA_DIR = ROOT / "crates" / "ledkit-text" / "src" / "faces" / "data"
FACES_JSON = ROOT / "site" / "text-preview" / "faces.json"

NAMES = ["4x6", "5x7", "5x8", "6x10", "6x13B", "7x13B", "7x14B", "8x13B", "9x15B", "9x18B", "10x20"]
RUST_NAMES = {name: "X11_" + name.upper().replace("B", "_BOLD") for name in NAMES}
RUST_NAMES["tiny"] = "TINY_3X5"
SIZES = [4, 5, 6, 8]  # 1x, 1.25x, 1.5x, 2x, in quarters
CODES = range(32, 127)
QUESTION = ord("?")


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


def read_tiny(path):
    """The tiny 3x5 face in a 4x5 cell (its shape plus a blank column), capitals only."""
    glyphs = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        char, bits = line[0], line[2:]
        glyphs[ord(char)] = [bits[y * 3 : y * 3 + 3] + "0" for y in range(5)]
    for code in range(ord("a"), ord("z") + 1):  # lower case shows as capitals
        glyphs[code] = glyphs[code - 32]
    return {"w": 4, "h": 5, "ascent": 5, "glyphs": glyphs}


def load_faces():
    """All faces, in output order: the BDF names, then tiny."""
    faces = {name: read_bdf(BDF_DIR / f"{name}.bdf") for name in NAMES}
    faces["tiny"] = read_tiny(TINY_FILE)
    return faces


def scale2x(grid):
    """EPX / Scale2x: doubles a 0/1 grid and rounds off diagonal stair-steps. Outside is 0."""
    h, w = len(grid), len(grid[0])

    def at(x, y):
        return grid[y][x] if 0 <= x < w and 0 <= y < h else 0

    out = [[0] * (w * 2) for _ in range(h * 2)]
    for y in range(h):
        for x in range(w):
            p, a, b, c, d = at(x, y), at(x, y - 1), at(x + 1, y), at(x - 1, y), at(x, y + 1)
            e0 = e1 = e2 = e3 = p
            if c == a and c != d and a != b:
                e0 = a
            if a == b and a != c and b != d:
                e1 = b
            if d == c and d != b and c != a:
                e2 = c
            if b == d and b != a and d != c:
                e3 = d
            out[2 * y][2 * x], out[2 * y][2 * x + 1] = e0, e1
            out[2 * y + 1][2 * x], out[2 * y + 1][2 * x + 1] = e2, e3
    return out


def output_size(cell_w, cell_h, quarters):
    """Size in LEDs of a cell at quarters/4 x, rounding halves up (like JavaScript's Math.round)."""
    return (2 * cell_w * quarters + 4) // 8, (2 * cell_h * quarters + 4) // 8


def bake_glyph(rows, quarters):
    """One glyph cell (list of '0'/'1' strings) at quarters/4 x, as w*h bytes of 0..=16."""
    cell_h, cell_w = len(rows), len(rows[0])
    bitmap = [[int(c) for c in row] for row in rows]
    out_w, out_h = output_size(cell_w, cell_h, quarters)
    if quarters == 4:
        return bytes(16 * v for row in bitmap for v in row)
    hi = bitmap
    for _ in range(3):
        hi = scale2x(hi)
    data = bytearray()
    for y in range(out_h):
        for x in range(out_w):
            total = 0
            for j in range(4):
                # hy = floor((y + (j + 0.5) / 4) / s * 8) with s = out_h / cell_h, in integers
                hy = (8 * y + 2 * j + 1) * cell_h // out_h
                for i in range(4):
                    hx = (8 * x + 2 * i + 1) * cell_h // out_h
                    if hx < cell_w * 8 and hy < cell_h * 8:
                        total += hi[hy][hx]
            data.append(total)
    return bytes(data)


def bake_face(face):
    """A face as four sizes [{w, h, glyphs: [bytes x 95]}], glyphs for codes 32..=126."""
    sizes = []
    for quarters in SIZES:
        w, h = output_size(face["w"], face["h"], quarters)
        glyphs = []
        for code in CODES:
            rows = face["glyphs"].get(code) or face["glyphs"][QUESTION]
            glyphs.append(bake_glyph(rows, quarters))
        sizes.append({"w": w, "h": h, "glyphs": glyphs})
    return sizes


def face_bin(sizes):
    return b"".join(g for size in sizes for g in size["glyphs"])


def mod_rs(baked):
    lines = [
        "// Generated by tools/fontgen.py, do not edit.",
        "// Each .bin holds four sizes (1x, 1.25x, 1.5x, 2x), each 95 glyphs (codes 32..=126),",
        "// each w*h bytes: the sum of 16 samples per LED, 0..=16.",
        "",
        "use crate::faces::bitmap::{BakedSize, BitmapFace};",
        "",
        "const fn part(all: &'static [u8], start: usize, len: usize) -> &'static [u8] {",
        "    all.split_at(start).1.split_at(len).0",
        "}",
    ]
    for name, sizes in baked.items():
        rust = RUST_NAMES[name]
        lines += ["", f'const {rust}_BIN: &[u8] = include_bytes!("{name}.bin");', ""]
        lines += [f"pub(crate) static {rust}: BitmapFace = BitmapFace {{"]
        lines += [f'    name: "{name}",', "    sizes: ["]
        start = 0
        for size in sizes:
            length = 95 * size["w"] * size["h"]
            lines += [
                "        BakedSize {",
                f"            width: {size['w']},",
                f"            height: {size['h']},",
                f"            data: part({rust}_BIN, {start}, {length}),",
                "        },",
            ]
            start += length
        lines += ["    ],", "};"]
    return "\n".join(lines) + "\n"


def faces_json(baked):
    out = {
        name: {
            "sizes": [
                {"w": s["w"], "h": s["h"], "glyphs": ["".join(chr(97 + b) for b in g) for g in s["glyphs"]]}
                for s in sizes
            ]
        }
        for name, sizes in baked.items()
    }
    return json.dumps(out, separators=(",", ":")) + "\n"


def outputs():
    """Every generated file as {path: bytes}."""
    baked = {name: bake_face(face) for name, face in load_faces().items()}
    files = {DATA_DIR / f"{name}.bin": face_bin(sizes) for name, sizes in baked.items()}
    files[DATA_DIR / "mod.rs"] = mod_rs(baked).encode("utf-8")
    files[FACES_JSON] = faces_json(baked).encode("utf-8")
    return files


def main():
    files = outputs()
    if "--check" in sys.argv:
        stale = [p for p, data in files.items() if not p.exists() or p.read_bytes() != data]
        if stale:
            names = ", ".join(str(p.relative_to(ROOT)) for p in stale)
            sys.exit(f"out of date: {names}\nrun: python tools/fontgen.py")
        print("generated font files are up to date")
        return
    for path, data in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    total = sum(len(d) for p, d in files.items() if p.suffix == ".bin")
    print(f"wrote {len(files)} files ({total} bytes of .bin)")


if __name__ == "__main__":
    main()
