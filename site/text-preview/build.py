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
sys.path.insert(0, str(HERE.parent.parent / "tools"))
from fontgen import BDF_DIR, NAMES, read_bdf  # noqa: E402


def build():
    fonts = {name: read_bdf(BDF_DIR / f"{name}.bdf") for name in NAMES}
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
