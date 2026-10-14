import { decodePNG, encodePNG } from '../png.mjs';
const names = ['base', 'thinking', 'read'];
const f = 3, gap = 10;
const imgs = names.map(n => decodePNG(`.fool-work/mascot/anchor/seat-${n}.png`));
const cw = 150, ch = 118, cx = imgs[0].width - cw - 4, cy = 14;
const W = (cw * f + gap) * imgs.length + gap, H = ch * f + 28;
const out = Buffer.alloc(W * H * 4);
for (let i = 0; i < W * H; i++) { out[i*4] = 14; out[i*4+1] = 12; out[i*4+2] = 30; out[i*4+3] = 255; }
imgs.forEach((im, k) => {
  const ox = gap + k * (cw * f + gap);
  for (let y = 0; y < ch * f; y++) for (let x = 0; x < cw * f; x++) {
    const sx = cx + Math.floor(x / f), sy = cy + Math.floor(y / f);
    if (sx < 0 || sy < 0 || sx >= im.width || sy >= im.height) continue;
    const s = (sy * im.width + sx) * 4, o = ((y + 18) * W + ox + x) * 4;
    out[o]=im.rgba[s]; out[o+1]=im.rgba[s+1]; out[o+2]=im.rgba[s+2]; out[o+3]=255;
  }
});
encodePNG({ width: W, height: H, rgba: out }, '.fool-work/mascot/anchor/seat-zoom3.png');
console.log(W, H, names.join(' | '));
