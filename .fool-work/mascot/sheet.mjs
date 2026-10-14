import { decodePNG, encodePNG } from './png.mjs';
const names = ['base','dissatisfied','read','thinking','smirk'];
const f = 2, gap = 4;
const imgs = names.map(n => decodePNG(`assets/mascot/${n}.png`));
const w = imgs[0].width, h = imgs[0].height;
const W = (w*f + gap) * imgs.length, H = h*f;
const out = Buffer.alloc(W*H*4);
for (let i = 0; i < W*H; i++) { out[i*4]=70; out[i*4+1]=70; out[i*4+2]=80; out[i*4+3]=255; }
imgs.forEach((im, k) => {
  const ox = k*(w*f+gap);
  for (let y=0;y<h*f;y++) for (let x=0;x<w*f;x++) {
    const s=(Math.floor(y/f)*w+Math.floor(x/f))*4, o=((y)*W+ox+x)*4;
    if (im.rgba[s+3]===0) continue;
    out[o]=im.rgba[s]; out[o+1]=im.rgba[s+1]; out[o+2]=im.rgba[s+2]; out[o+3]=255;
  }
});
encodePNG({width:W,height:H,rgba:out}, '.fool-work/mascot/preview-sheet.png');
console.log(W, H, names.join(' | '));
