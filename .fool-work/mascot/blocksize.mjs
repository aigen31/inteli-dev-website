import { decodePNG } from './png.mjs';
const root = '/home/aigen31/.dsh/attachments/v1/objects/';
const base = root + '94/94f4e8b5ce255986a3960adb8df327365958786a38533e9a2f296f0f1bf5a652';
const { width: w, height: h, rgba } = decodePNG(base);
function px(x, y) { const i = (y*w+x)*4; return [rgba[i],rgba[i+1],rgba[i+2],rgba[i+3]]; }
for (const k of [2,3,4]) {
  const nw = Math.floor(w/k), nh = Math.floor(h/k);
  let err = 0, n = 0, alphaErr = 0;
  for (let y = 0; y < nh; y++) for (let x = 0; x < nw; x++) {
    const c = px(x*k + (k>>1), y*k + (k>>1));
    for (let dy = 0; dy < k; dy++) for (let dx = 0; dx < k; dx++) {
      const o = px(x*k+dx, y*k+dy);
      if ((o[3] > 127) !== (c[3] > 127)) alphaErr++;
      else if (o[3] > 127) { err += Math.abs(o[0]-c[0]) + Math.abs(o[1]-c[1]) + Math.abs(o[2]-c[2]); n += 3; }
    }
  }
  console.log(`k=${k}: native ${nw}x${nh}, alpha mismatch ${alphaErr}px, mean rgb err ${(err/n).toFixed(2)}`);
}
