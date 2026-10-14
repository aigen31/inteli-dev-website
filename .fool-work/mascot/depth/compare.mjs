import { decodePNG, encodePNG } from '../png.mjs';
const [a, b, out, xs, ys, ws, hs, fs] = process.argv.slice(2);
const A = decodePNG(a), B = decodePNG(b);
const X = +xs, Y = +ys, W = +ws, H = +hs, F = +fs;
const total = W * F * 2 + 24, HH = H * F + 12;
const buf = Buffer.alloc(total * HH * 4);
for (let i = 0; i < total * HH; i++) { buf[i*4]=10; buf[i*4+1]=8; buf[i*4+2]=20; buf[i*4+3]=255; }
[[A, 6], [B, W * F + 18]].forEach(([im, ox]) => {
  for (let y = 0; y < H * F; y++) for (let x = 0; x < W * F; x++) {
    const sx = X + Math.floor(x / F), sy = Y + Math.floor(y / F);
    if (sx >= im.width || sy >= im.height) continue;
    const o = ((y + 6) * total + ox + x) * 4, s = (sy * im.width + sx) * 4;
    buf[o]=im.rgba[s]; buf[o+1]=im.rgba[s+1]; buf[o+2]=im.rgba[s+2]; buf[o+3]=255;
  }
});
encodePNG({ width: total, height: HH, rgba: buf }, out);
console.log(out, total + 'x' + HH);
