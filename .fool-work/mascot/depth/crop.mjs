import { decodePNG, encodePNG } from '../png.mjs';
const [src, out, x0, y0, w, h, f] = process.argv.slice(2);
const im = decodePNG(src);
const X0 = +x0, Y0 = +y0, W = +w, H = +h, F = +f || 4;
const buf = Buffer.alloc(W * F * H * F * 4);
for (let y = 0; y < H * F; y++) for (let x = 0; x < W * F; x++) {
  const sx = X0 + Math.floor(x / F), sy = Y0 + Math.floor(y / F);
  const o = (y * W * F + x) * 4;
  if (sx < 0 || sy < 0 || sx >= im.width || sy >= im.height) { buf[o] = 255; buf[o+3] = 255; continue; }
  const s = (sy * im.width + sx) * 4;
  buf[o] = im.rgba[s]; buf[o+1] = im.rgba[s+1]; buf[o+2] = im.rgba[s+2]; buf[o+3] = 255;
}
encodePNG({ width: W * F, height: H * F, rgba: buf }, out);
console.log(out, W * F + 'x' + H * F);
