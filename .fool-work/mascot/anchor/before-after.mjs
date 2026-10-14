import { decodePNG, encodePNG } from '../png.mjs';
const before = decodePNG('/home/aigen31/.fool-work/mascot/anchor/now-thinking.png');
const after = decodePNG('.fool-work/mascot/anchor/seat-thinking.png');
// «до» — целое окно 511x361, «после» — кроп 538x140 полосы ввода
const f = 3;
function crop(im, x, y, w, h) { return { im, x, y, w, h }; }
// в «до» маскот примерно x 330..500, y 110..260; линия строки ввода — y≈226
const panels = [crop(before, 320, 110, 180, 150), crop(after, 330, 30, 180, 110)];
const gap = 8;
const W = panels.reduce((a, p) => a + p.w * f + gap, gap);
const H = Math.max(...panels.map(p => p.h * f)) + 24;
const out = Buffer.alloc(W * H * 4);
for (let i = 0; i < W * H; i++) { out[i*4]=14; out[i*4+1]=12; out[i*4+2]=30; out[i*4+3]=255; }
let ox = gap;
for (const p of panels) {
  for (let y = 0; y < p.h * f; y++) for (let x = 0; x < p.w * f; x++) {
    const sx = p.x + Math.floor(x / f), sy = p.y + Math.floor(y / f);
    if (sx < 0 || sy < 0 || sx >= p.im.width || sy >= p.im.height) continue;
    const s = (sy * p.im.width + sx) * 4, o = ((y + 20) * W + ox + x) * 4;
    out[o]=p.im.rgba[s]; out[o+1]=p.im.rgba[s+1]; out[o+2]=p.im.rgba[s+2]; out[o+3]=255;
  }
  ox += p.w * f + gap;
}
encodePNG({ width: W, height: H, rgba: out }, '.fool-work/mascot/anchor/thinking-before-after.png');
console.log(W, H);
