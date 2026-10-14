import zlib from 'node:zlib';
import fs from 'node:fs';
const SIG = Buffer.from([0x89,0x50,0x4e,0x47,0x0d,0x0a,0x1a,0x0a]);
export function decodePNG(file) {
  const buf = fs.readFileSync(file);
  if (!buf.subarray(0,8).equals(SIG)) throw new Error('not png: ' + file);
  let off = 8, ihdr = null, idat = [], plte = null, trns = null;
  while (off < buf.length) {
    const len = buf.readUInt32BE(off);
    const type = buf.toString('ascii', off+4, off+8);
    const data = buf.subarray(off+8, off+8+len);
    if (type === 'IHDR') ihdr = { width: data.readUInt32BE(0), height: data.readUInt32BE(4), depth: data[8], color: data[9], interlace: data[12] };
    else if (type === 'IDAT') idat.push(data);
    else if (type === 'PLTE') plte = Buffer.from(data);
    else if (type === 'tRNS') trns = Buffer.from(data);
    off += 12 + len;
  }
  if (!ihdr) throw new Error('no IHDR');
  if (ihdr.depth !== 8 || ihdr.interlace) throw new Error('unsupported png');
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const ch = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[ihdr.color];
  const { width: w, height: h } = ihdr;
  const stride = w * ch;
  const out = Buffer.alloc(stride * h);
  let prev = Buffer.alloc(stride);
  for (let y = 0; y < h; y++) {
    const ft = raw[y * (stride + 1)];
    const cur = Buffer.from(raw.subarray(y * (stride + 1) + 1, y * (stride + 1) + 1 + stride));
    for (let i = 0; i < stride; i++) {
      const a = i >= ch ? cur[i - ch] : 0, b = prev[i], c = i >= ch ? prev[i - ch] : 0;
      if (ft === 1) cur[i] = (cur[i] + a) & 255;
      else if (ft === 2) cur[i] = (cur[i] + b) & 255;
      else if (ft === 3) cur[i] = (cur[i] + ((a + b) >> 1)) & 255;
      else if (ft === 4) { const p = a + b - c, pa = Math.abs(p-a), pb = Math.abs(p-b), pc = Math.abs(p-c);
        cur[i] = (cur[i] + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c)) & 255; }
      else if (ft !== 0) throw new Error('filter ' + ft);
    }
    cur.copy(out, y * stride); prev = cur;
  }
  const rgba = Buffer.alloc(w * h * 4);
  for (let i = 0; i < w * h; i++) {
    if (ihdr.color === 6) out.copy(rgba, i*4, i*4, i*4+4);
    else if (ihdr.color === 2) { out.copy(rgba, i*4, i*3, i*3+3); rgba[i*4+3] = 255; }
    else if (ihdr.color === 0) { const v = out[i]; rgba.fill(v, i*4, i*4+3); rgba[i*4+3] = 255; }
    else if (ihdr.color === 4) { const v = out[i*2]; rgba.fill(v, i*4, i*4+3); rgba[i*4+3] = out[i*2+1]; }
    else if (ihdr.color === 3) { const p = out[i]*3; rgba[i*4]=plte[p]; rgba[i*4+1]=plte[p+1]; rgba[i*4+2]=plte[p+2]; rgba[i*4+3] = trns && trns[out[i]] != null ? trns[out[i]] : 255; }
  }
  return { width: w, height: h, rgba };
}
function crc32(buf) {
  let c, table = crc32.table;
  if (!table) { table = crc32.table = new Int32Array(256);
    for (let n = 0; n < 256; n++) { c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; table[n] = c; } }
  let crc = -1;
  for (let i = 0; i < buf.length; i++) crc = (crc >>> 8) ^ table[(crc ^ buf[i]) & 255];
  return (crc ^ -1) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}
export function encodePNG({ width, height, rgba }, file) {
  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) { raw[y * (stride + 1)] = 0; rgba.copy(raw, y * (stride + 1) + 1, y * stride, y * stride + stride); }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0); ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; ihdr[9] = 6;
  const png = Buffer.concat([SIG, chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw, { level: 9 })), chunk('IEND', Buffer.alloc(0))]);
  if (file) fs.writeFileSync(file, png);
  return png;
}
