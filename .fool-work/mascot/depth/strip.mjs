import { decodePNG, encodePNG } from '../png.mjs';
const names = process.argv.slice(2);
const f = 2, gap = 6, x0 = 300, y0 = 30, w = 235, h = 115;
const imgs = names.map(n => decodePNG(`.fool-work/mascot/depth/now-${n}.png`));
const W = (w * f + gap) * imgs.length + gap, H = h * f + 12;
const buf = Buffer.alloc(W * H * 4);
for (let i = 0; i < W * H; i++) { buf[i*4]=10; buf[i*4+1]=8; buf[i*4+2]=20; buf[i*4+3]=255; }
imgs.forEach((im, k) => {
  const ox = gap + k * (w * f + gap);
  for (let y = 0; y < h * f; y++) for (let x = 0; x < w * f; x++) {
    const sx = x0 + Math.floor(x / f), sy = y0 + Math.floor(y / f);
    if (sx >= im.width || sy >= im.height) continue;
    const o = ((y + 6) * W + ox + x) * 4, s = (sy * im.width + sx) * 4;
    buf[o]=im.rgba[s]; buf[o+1]=im.rgba[s+1]; buf[o+2]=im.rgba[s+2]; buf[o+3]=255;
  }
});
encodePNG({ width: W, height: H, rgba: buf }, '.fool-work/mascot/depth/now-strip.png');
console.log(W, H, names.join(' | '));
