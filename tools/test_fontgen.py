"""Tests for tools/fontgen.py. Run: python tools/test_fontgen.py"""

import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import fontgen  # noqa: E402


def sizes_of(name):
    return fontgen.bake_face(fontgen.load_faces()[name])


def glyph(size, code):
    return size["glyphs"][code - 32]


class FontgenTests(unittest.TestCase):
    def test_one_x_is_the_font(self):
        face = fontgen.load_faces()["5x8"]
        rows = face["glyphs"][ord("A")]
        one_x = sizes_of("5x8")[0]
        self.assertEqual((one_x["w"], one_x["h"]), (5, 8))
        want = bytes(16 if c == "1" else 0 for row in rows for c in row)
        self.assertEqual(glyph(one_x, ord("A")), want)
        self.assertIn(16, want)

    def test_sizes(self):
        got = [(s["w"], s["h"]) for s in sizes_of("5x8")]
        self.assertEqual(got, [(5, 8), (6, 10), (8, 12), (10, 16)])
        self.assertEqual((round(5 * 1.25), round(8 * 1.25)), (6, 10))

    def test_sizes_round_half_up(self):
        # 7 * 1.5 = 10.5 must give 11 (as the preview's Math.round), not 10.
        got = [(s["w"], s["h"]) for s in sizes_of("7x13B")]
        self.assertEqual(got[2], (11, 20))

    def test_scale2x_rounds_a_diagonal(self):
        got = fontgen.scale2x([[1, 0], [0, 1]])
        self.assertEqual(got, [[1, 1, 0, 0], [1, 1, 1, 0], [0, 1, 1, 1], [0, 0, 1, 1]])

    def test_isolated_glyph(self):
        faces = fontgen.load_faces()
        before = glyph(fontgen.bake_face(faces["5x8"])[1], ord("A"))
        faces["5x8"]["glyphs"][ord("B")] = ["11111"] * 8
        after = glyph(fontgen.bake_face(faces["5x8"])[1], ord("A"))
        self.assertEqual(before, after)
        # outside the cell counts as 0, so Scale2x rounds the corners of a lone pixel:
        # by symmetry its four LEDs match, and they are not fully lit
        lone = fontgen.bake_glyph(["1"], 8)
        self.assertEqual(len(set(lone)), 1)
        self.assertEqual(lone[0], 11)

    def test_missing_is_question_mark(self):
        faces = fontgen.load_faces()
        for name in ("5x8", "tiny"):
            for size in fontgen.bake_face(faces[name]):
                self.assertEqual(len(size["glyphs"]), 95)
        tiny = sizes_of("tiny")
        for size in tiny:
            self.assertEqual(glyph(size, ord("a")), glyph(size, ord("A")))
            self.assertEqual(glyph(size, ord("@")), glyph(size, ord("?")))
        # a code the BDF lacks gets '?'
        faces = fontgen.load_faces()
        del faces["5x8"]["glyphs"][ord("~")]
        baked = fontgen.bake_face(faces["5x8"])
        for size in baked:
            self.assertEqual(glyph(size, ord("~")), glyph(size, ord("?")))

    def test_tiny_cell_has_a_blank_column(self):
        one_x = sizes_of("tiny")[0]
        self.assertEqual((one_x["w"], one_x["h"]), (4, 5))
        g = glyph(one_x, ord("A"))
        self.assertEqual([g[y * 4 + 3] for y in range(5)], [0] * 5)

    def test_range(self):
        for name in fontgen.load_faces():
            for size in sizes_of(name):
                for g in size["glyphs"]:
                    self.assertEqual(len(g), size["w"] * size["h"])
                    self.assertTrue(all(0 <= b <= 16 for b in g))


if __name__ == "__main__":
    unittest.main()
