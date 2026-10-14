import { decodePNG, encodePNG } from './png.mjs';
const [src, dst, factorArg, bg] = process.argv.slice(2);
const factor = Number(factorArg || 4);
const { width: w, height: h, rgba } = decodePNG(src);
const W = w * factor, H = h * factor;
const out = Buffer.alloc(W * H * 4);
for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
  const i = (Math.floor(y / factor) * w + Math.floor(x / factor)) * 4;
  const o = (y * W + x) * 4;
  if (rgba[i+3] === 0) {
    // шахматка вместо прозрачности, чтобы видеть края
    const c = (Math.floor(x / factor / 6) + Math.floor(y / factor / 6)) % 2 === 0 ? 60 : 90;
    out[o] = c; out[o+1] = c; out[o+2] = c + 10; out[o+3] = 255;
  } else { out[o] = rgba[i]; out[o+1] = rgba[i+1]; out[o+2] = rgba[i+2]; out[o+3] = 255; }
}
encodePNG({ width: W, height: H, rgba: out }, dst);
console.log(dst, `${W}x${H}`);
