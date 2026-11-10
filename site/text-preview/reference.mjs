// Renders reference cases from glyphs.js (the preview's segment drawing) into
// crates/ledkit-text/src/faces/segments.fixtures, which the Rust tests read to check
// their integer ports. Values are coverage in 64ths (unsharpened 8x downsample).
// Usage: node reference.mjs [--check]
import vm from 'node:vm';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.join(here, '..', '..', 'crates', 'ledkit-text', 'src', 'faces', 'segments.fixtures');
const src = fs.readFileSync(path.join(here, 'glyphs.js'), 'utf8');
const { renderSeg, renderSeg14 } = vm.runInNewContext(src + '\n({ renderSeg, renderSeg14 })');

const HEIGHTS = [5, 8, 10, 16, 30, 40];
const TEXTS = ['0123456789', '12:34', '3.5'];
const TEXTS14 = [...TEXTS, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', '?!-+/'];

const cases = [];
const add = (name, height, text, bmp, hard) => {
  const lines = [`case ${name} height=${height} text=${text}`, `${bmp.w} ${bmp.h}`];
  for (let y = 0; y < bmp.h; y++) {
    const row = [];
    for (let x = 0; x < bmp.w; x++) {
      const v = bmp.px[y * bmp.w + x];
      row.push(hard ? (v ? 64 : 0) : Math.round(v * 64));
    }
    lines.push(row.join(' '));
  }
  cases.push(lines.join('\n'));
};

for (const [name, slant] of [['seg7', false], ['seg7slant', true]]) {
  for (const H of HEIGHTS) for (const t of TEXTS) add(name, H, t, renderSeg(t, H, slant, true, null), false);
}
for (const [name, slant] of [['seg7hard', false], ['seg7slanthard', true]]) {
  for (const H of HEIGHTS) for (const t of TEXTS) add(name, H, t, renderSeg(t, H, slant, false), true);
}
for (const H of [7, ...HEIGHTS.slice(1)]) {
  for (const t of TEXTS14) add('seg14', H, t, renderSeg14(t, H, true, null), false);
}

const text = cases.join('\n\n') + '\n';
if (process.argv.includes('--check')) {
  const disk = fs.existsSync(out) ? fs.readFileSync(out, 'utf8') : null;
  if (disk !== text) { console.error('segments.fixtures is stale: run node site/text-preview/reference.mjs'); process.exit(1); }
  console.log('segments.fixtures up to date');
} else {
  fs.writeFileSync(out, text);
  console.log(`wrote ${out} (${text.length} bytes, ${cases.length} cases)`);
}
