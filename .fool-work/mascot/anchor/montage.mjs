import { decodePNG, encodePNG } from '../png.mjs';
const names = ['base', 'read', 'thinking', 'smirk', 'dissatisfied'];
const imgs = names.map(n => decodePNG(`.fool-work/mascot/anchor/seat-${n}.png`));
const gap = 8;
const W = imgs.reduce((a, i) => a + i.width + gap, gap);
const H = Math.max(...imgs.map(i => i.height)) + 24;
const out = Buffer.alloc(W * H * 4);
for (let i = 0; i < W * H; i++) { out[i*4] = 12; out[i*4+1] = 10; out[i*4+2] = 26; out[i*4+3] = 255; }
let x0 = gap;
for (const im of imgs) {
  for (let y = 0; y < im.height; y++) for (let x = 0; x < im.width; x++) {
    const s = (y*im.width+x)*4, o = ((y+20)*W + x0 + x)*4;
    out[o]=im.rgba[s]; out[o+1]=im.rgba[s+1]; out[o+2]=im.rgba[s+2]; out[o+3]=255;
  }
  x0 += im.width + gap;
}
encodePNG({ width: W, height: H, rgba: out }, '.fool-work/mascot/anchor/seat-montage.png');
console.log(W, H);
