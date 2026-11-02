// DOM-free glyph drawing shared by the preview page and (later) the Rust reference tests.
// Inlined into the page by build.py; must not touch document, window or $.

// A bitmap is {w, h, px: Float32Array} with values 0..1 (brightness of text there).
function bitmap(w, h) { return { w, h, px: new Float32Array(w * h) }; }

// 7-segment digits: segments a..g as rectangles, scaled to height H.
// The usual 7-segment alphabet. Some letters only exist as lower case (b, d, n, r, t ...);
// K, M, V, W and X have no readable 7-segment form and show blank.
const SEGS = { '0':'abcdef','1':'bc','2':'abdeg','3':'abcdg','4':'bcfg','5':'acdfg','6':'acdefg','7':'abc','8':'abcdefg','9':'abcdfg',
  '-':'g','_':'d','=':'dg',' ':'','°':'abfg',
  'A':'abcefg','B':'cdefg','C':'adef','D':'bcdeg','E':'adefg','F':'aefg','G':'acdef','H':'bcefg','I':'ef','J':'bcde','L':'def',
  'N':'ceg','O':'abcdef','P':'abefg','Q':'abcfg','R':'eg','S':'acdfg','T':'defg','U':'bcdef','Y':'bcdfg','Z':'abdeg',
  'a':'abcdeg','b':'cdefg','c':'deg','d':'bcdeg','e':'abdefg','f':'aefg','g':'abcdfg','h':'cefg','i':'c','j':'bcd','l':'ef',
  'n':'ceg','o':'cdeg','p':'abefg','q':'abcfg','r':'eg','s':'acdfg','t':'defg','u':'cde','y':'bcdfg','z':'abdeg' };
// aa with k === null returns the plain 8x-downsampled coverage (no sharpening).
function renderSeg(text, H, slant, aa, k) {
  if (aa) { const ss = 8, o = downsample(renderSeg(text, H * ss, slant, false), ss); if (k !== null) o.px = o.px.map(v => sharpen(v, k)); return o; }
  H = Math.max(5, H);
  const t = Math.max(1, Math.round(H / 7));            // stroke thickness
  const W = Math.max(3, Math.round(H * 0.55));
  const gap = Math.max(1, Math.round(t * 0.8));
  const chars = [...text].map(c => SEGS[c] !== undefined || c === ':' || c === '.' ? c : c.toUpperCase());
  const widths = chars.map(c => (c === ':' || c === '.') ? t + 2 * gap : W);
  const skew = slant ? Math.round(H * 0.18) : 0;
  const total = widths.reduce((a, w) => a + w + gap * 2, 0) + skew;
  const b = bitmap(Math.max(1, total), H);
  const mid = Math.floor((H - t) / 2);
  const put = (x, y) => { const sx = x + (slant ? Math.round(skew * (1 - y / (H - 1))) : 0); if (sx >= 0 && sx < b.w && y >= 0 && y < b.h) b.px[y * b.w + sx] = 1; };
  const rect = (x0, y0, w, h) => { for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) put(x, y); };
  let x = 0;
  chars.forEach((c, i) => {
    if (c === ':') { rect(x + gap, Math.round(H * 0.28) - (t >> 1), t, t); rect(x + gap, Math.round(H * 0.72) - (t >> 1), t, t); }
    else if (c === '.') rect(x + gap, H - t, t, t);
    else {
      const s = SEGS[c] || '';
      const v = mid - t;            // length of the upper verticals
      const v2 = H - mid - 2 * t;   // lower verticals
      if (s.includes('a')) rect(x + t, 0, W - 2 * t, t);
      if (s.includes('g')) rect(x + t, mid, W - 2 * t, t);
      if (s.includes('d')) rect(x + t, H - t, W - 2 * t, t);
      if (s.includes('f')) rect(x, t, t, v);
      if (s.includes('b')) rect(x + W - t, t, t, v);
      if (s.includes('e')) rect(x, mid + t, t, v2);
      if (s.includes('c')) rect(x + W - t, mid + t, t, v2);
    }
    x += widths[i] + gap * 2;
  });
  return b;
}

// 14-segment ("starburst") display: every capital letter has a shape.
// Segments: a top, b/c right, d bottom, e/f left, g1/g2 middle halves, i/l centre verticals,
// h/j upper diagonals, k/m lower diagonals.
const SEG14 = {
  'A':'a b c e f g1 g2','B':'a b c d i l g2','C':'a d e f','D':'a b c d i l','E':'a d e f g1','F':'a e f g1',
  'G':'a c d e f g2','H':'b c e f g1 g2','I':'a d i l','J':'b c d e','K':'e f g1 j m','L':'d e f','M':'b c e f h j',
  'N':'b c e f h m','O':'a b c d e f','P':'a b e f g1 g2','Q':'a b c d e f m','R':'a b e f g1 g2 m','S':'a c d f g1 g2',
  'T':'a i l','U':'b c d e f','V':'e f k j','W':'b c e f k m','X':'h j k m','Y':'h j l','Z':'a d j k',
  '0':'a b c d e f j k','1':'b c j','2':'a b d e g1 g2','3':'a b c d g2','4':'b c f g1 g2','5':'a c d f g1 g2',
  '6':'a c d e f g1 g2','7':'a b c','8':'a b c d e f g1 g2','9':'a b c d f g1 g2',
  '-':'g1 g2','+':'g1 g2 i l','/':'j k','*':'g1 g2 h i j k l m','\'':'i','"':'f i','_':'d','=':'d g1 g2',
  '<':'j m','>':'h k','?':'a b g2 l','!':'b c','$':'a c d f g1 g2 i l','%':'c f j k g1 g2',' ':''
};
function renderSeg14(text, H, aa, k) {
  H = Math.max(7, H);
  const ss = 8, HH = H * ss;                     // draw big, then shrink
  const t = Math.max(1, Math.round(H / 8)) * ss; // stroke thickness
  const W = Math.round(H * 0.62) * ss, gapc = Math.round(t * 0.9);
  const chars = [...text.toUpperCase()];
  const widths = chars.map(c => (c === ':' || c === '.' || c === ',') ? t + 2 * gapc : W);
  const total = widths.reduce((s, w) => s + w + 2 * gapc, 0);
  const big = bitmap(Math.ceil(total / ss) * ss, HH);
  const h = t / 2;
  const caps = [];
  const P = (x0, w) => ({ L: x0 + h, R: x0 + w - h, C: x0 + w / 2, T: h, M: HH / 2, B: HH - h });
  let x0 = 0;
  chars.forEach((c, n) => {
    const p = P(x0, widths[n]);
    const add = (ax, ay, bx, by, shrink) => {  // capsule, pulled back from its ends so segments don't touch
      const dx = bx - ax, dy = by - ay, len = Math.hypot(dx, dy), s = shrink / len;
      caps.push([ax + dx * s, ay + dy * s, bx - dx * s, by - dy * s]);
    };
    if (c === ':') { caps.push([p.C, HH * 0.3, p.C, HH * 0.3]); caps.push([p.C, HH * 0.7, p.C, HH * 0.7]); }
    else if (c === '.' || c === ',') caps.push([p.C, p.B, p.C, p.B]);
    else for (const s of (SEG14[c] ?? '').split(' ').filter(Boolean)) {
      const g = gapc, d = gapc * 1.6;
      ({
        a: () => add(p.L, p.T, p.R, p.T, g), d: () => add(p.L, p.B, p.R, p.B, g),
        f: () => add(p.L, p.T, p.L, p.M, g), e: () => add(p.L, p.M, p.L, p.B, g),
        b: () => add(p.R, p.T, p.R, p.M, g), c: () => add(p.R, p.M, p.R, p.B, g),
        g1: () => add(p.L, p.M, p.C, p.M, g), g2: () => add(p.C, p.M, p.R, p.M, g),
        i: () => add(p.C, p.T, p.C, p.M, g), l: () => add(p.C, p.M, p.C, p.B, g),
        h: () => add(p.L, p.T, p.C, p.M, d), j: () => add(p.R, p.T, p.C, p.M, d),
        k: () => add(p.L, p.B, p.C, p.M, d), m: () => add(p.R, p.B, p.C, p.M, d),
      })[s]();
    }
    x0 += widths[n] + 2 * gapc;
  });
  for (const [ax, ay, bx, by] of caps) {
    const minx = Math.max(0, Math.floor(Math.min(ax, bx) - h)), maxx = Math.min(big.w - 1, Math.ceil(Math.max(ax, bx) + h));
    const miny = Math.max(0, Math.floor(Math.min(ay, by) - h)), maxy = Math.min(big.h - 1, Math.ceil(Math.max(ay, by) + h));
    const dx = bx - ax, dy = by - ay, l2 = dx * dx + dy * dy || 1;
    for (let y = miny; y <= maxy; y++) for (let x = minx; x <= maxx; x++) {
      const u = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / l2));
      if (Math.hypot(x - ax - u * dx, y - ay - u * dy) <= h) big.px[y * big.w + x] = 1;
    }
  }
  const o = downsample(big, ss);
  if (aa) { if (k !== null) o.px = o.px.map(v => sharpen(v, k)); }
  else o.px = o.px.map(v => (v >= 0.5 ? 1 : 0));
  return o;
}

// Average each k x k block down to one LED: edge LEDs get partial brightness.
function downsample(b, k) {
  const o = bitmap(Math.ceil(b.w / k), Math.ceil(b.h / k));
  for (let y = 0; y < b.h; y++) for (let x = 0; x < b.w; x++) o.px[((y / k) | 0) * o.w + ((x / k) | 0)] += b.px[y * b.w + x] / (k * k);
  return o;
}
// Push part-covered LEDs toward fully on or off, leaving a thinner soft rim. k is the edge sharpness.
function sharpen(v, k) { return Math.min(1, Math.max(0, (v - 0.5) * k + 0.5)); }
