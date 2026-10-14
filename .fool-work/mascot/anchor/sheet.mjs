import { decodePNG, encodePNG } from '../png.mjs';
import fs from 'node:fs';

const names = ['base', 'read', 'thinking', 'smirk', 'dissatisfied'];
const f = 2, gap = 6;
const imgs = names.map(n => decodePNG(`assets/mascot/${n}.png`));
const w = imgs[0].width, h = imgs[0].height;

// «Линия таза» — последняя строка широкой части силуэта перед обрывом к ногам.
function seatRow(rgba, w, h) {
  const spans = [];
  for (let y = 0; y < h; y++) {
    let minX = w, maxX = -1;
    for (let x = 0; x < w; x++) if (rgba[(y * w + x) * 4 + 3] > 128) { if (x < minX) minX = x; if (x > maxX) maxX = x; }
    spans.push(maxX < 0 ? 0 : maxX - minX + 1);
  }
  const maxSpan = Math.max(...spans);
  let seat = h;
  for (let y = Math.floor(h * 0.45); y < h - 1; y++) {
    if (spans[y] >= 0.55 * maxSpan && spans[y + 1] < spans[y] * 0.75) seat = y;
  }
  return { seat, spans, maxSpan };
}

const W = (w * f + gap) * imgs.length, H = h * f + 26;
const out = Buffer.alloc(W * H * 4, 0);
for (let i = 0; i < W * H; i++) { out[i*4] = 26; out[i*4+1] = 22; out[i*4+2] = 48; out[i*4+3] = 255; }

const report = [];
imgs.forEach((im, k) => {
  const ox = k * (w * f + gap);
  const { seat } = seatRow(im.rgba, w, h);
  report.push([names[k], seat, Math.round(100 * seat / h)]);
  for (let y = 0; y < h * f; y++) for (let x = 0; x < w * f; x++) {
    const s = (Math.floor(y / f) * w + Math.floor(x / f)) * 4, o = (y * W + ox + x) * 4;
    if (im.rgba[s + 3] === 0) continue;
    out[o] = im.rgba[s]; out[o + 1] = im.rgba[s + 1]; out[o + 2] = im.rgba[s + 2]; out[o + 3] = 255;
  }
  // опорная линия CSS: 80% высоты кадра — красная
  const cssY = Math.round(h * 0.8) * f;
  for (let x = 0; x < w * f; x++) { const o = (cssY * W + ox + x) * 4; out[o] = 255; out[o+1] = 60; out[o+2] = 60; }
  // фактическая линия таза кадра — зелёная
  const seatY = seat * f;
  for (let x = 0; x < w * f; x++) { const o = (seatY * W + ox + x) * 4; out[o] = 60; out[o+1] = 255; out[o+2] = 90; }
  // подпись полосой снизу: чем длиннее — тем ниже таз
  for (let x = 0; x < (seat - Math.round(h*0.8)) * 2 + 4; x++) for (let y = 0; y < 4; y++) {
    const o = ((h * f + 10 + y) * W + ox + x) * 4; out[o] = 255; out[o+1] = 220; out[o+2] = 0;
  }
});
encodePNG({ width: W, height: H, rgba: out }, '.fool-work/mascot/anchor/anchors.png');
console.log(report.map(([n, s, p]) => `${n}: таз y=${s} (${p}%), смещение от CSS-линии ${s - Math.round(192*0.8)}px`).join('\n'));
