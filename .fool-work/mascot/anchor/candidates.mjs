import { decodePNG, encodePNG } from '../png.mjs';
// одна картинка на кадр: сам спрайт + линии-кандидаты на разных высотах
const specs = [
  { name: 'read', rows: [154, 160, 165, 170], colors: [[255,80,80],[255,220,0],[80,255,120],[80,180,255]] },
  { name: 'thinking', rows: [154, 159, 164, 169], colors: [[255,80,80],[255,220,0],[80,255,120],[80,180,255]] },
];
const f = 3;
for (const spec of specs) {
  const im = decodePNG(`assets/mascot/${spec.name}.png`);
  const W = im.width * f, H = im.height * f + 20;
  const out = Buffer.alloc(W * H * 4);
  for (let i = 0; i < W*H; i++) { out[i*4]=16; out[i*4+1]=14; out[i*4+2]=34; out[i*4+3]=255; }
  for (let y = 0; y < im.height * f; y++) for (let x = 0; x < W; x++) {
    const s = (Math.floor(y/f) * im.width + Math.floor(x/f)) * 4, o = (y * W + x) * 4;
    if (im.rgba[s+3] === 0) continue;
    out[o]=im.rgba[s]; out[o+1]=im.rgba[s+1]; out[o+2]=im.rgba[s+2]; out[o+3]=255;
  }
  spec.rows.forEach((r, i) => {
    const c = spec.colors[i];
    for (let x = 0; x < W; x++) for (let t = 0; t < 2; t++) {
      const o = ((r * f + t) * W + x) * 4;
      out[o]=c[0]; out[o+1]=c[1]; out[o+2]=c[2]; out[o+3]=255;
    }
  });
  encodePNG({ width: W, height: H, rgba: out }, `.fool-work/mascot/anchor/cand-${spec.name}.png`);
  console.log(spec.name, spec.rows.join(','), '->', `.fool-work/mascot/anchor/cand-${spec.name}.png`);
}
