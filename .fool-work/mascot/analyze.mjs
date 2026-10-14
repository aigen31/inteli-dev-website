import { decodePNG } from './png.mjs';
const root = '/home/aigen31/.dsh/attachments/v1/objects/';
const files = {
  base: root + '94/94f4e8b5ce255986a3960adb8df327365958786a38533e9a2f296f0f1bf5a652',
  dissatisfied: root + '42/424c287a40c2622c742add4337adc1350c117d1083e87b230b9a2ba30d9107ac',
  read: root + 'ba/ba4d1c277251d20b23958aebd3a4c4ca16b76442f9f8f731902ebbeee84fe600',
  thinking: root + '02/023b0ef2973d0e849d459f130b47e33480ce7d5b68a412588fcbbe12ae1c6fdb',
  smirk: root + 'e0/e02d42cd524fd5cb2c5f390b8cc09ccd672a9a2910fd18abd1299ed8af437606',
};
for (const [name, f] of Object.entries(files)) {
  const { width: w, height: h, rgba } = decodePNG(f);
  let opaque = 0, semi = 0; const colors = new Set();
  let minX = w, minY = h, maxX = -1, maxY = -1;
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const i = (y*w+x)*4, a = rgba[i+3];
    if (a > 0) { opaque++; if (a < 255) semi++; }
    if (a > 8) { if (x<minX)minX=x; if (y<minY)minY=y; if (x>maxX)maxX=x; if (y>maxY)maxY=y; }
    if (a > 200) colors.add((rgba[i]<<16)|(rgba[i+1]<<8)|rgba[i+2]);
  }
  const runs = new Map();
  for (let y = 0; y < h; y++) {
    let run = 1;
    for (let x = 1; x <= w; x++) {
      const p = (y*w+x)*4, q = (y*w+x-1)*4;
      const same = x < w && rgba[p]===rgba[q] && rgba[p+1]===rgba[q+1] && rgba[p+2]===rgba[q+2] && rgba[p+3]===rgba[q+3];
      if (same) run++; else { if (rgba[q+3] > 200) runs.set(run, (runs.get(run)||0)+1); run = 1; }
    }
  }
  const top = [...runs.entries()].sort((a,b)=>b[1]-a[1]).slice(0,6);
  console.log(name.padEnd(13), `${w}x${h}`, 'alpha>0:', opaque, 'semi:', semi, 'colors:', colors.size,
    'bbox:', `${minX},${minY}..${maxX},${maxY}`, `(${maxX-minX+1}x${maxY-minY+1})`,
    'runs:', top.map(([l,c])=>`${l}:${c}`).join(' '));
}
