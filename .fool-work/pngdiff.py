#!/usr/bin/env python3
"""Мини-декодер PNG (RGBA/RGB, 8 бит, без интерлейса) + сравнение двух кадров.

Нужен, чтобы проверить измеримо, а не на глаз: рисуется ли canvas-сетка
поверх спрайта маскота. Считаем, в каких пикселях кадры различаются и
попадают ли эти пиксели в bbox спрайта.
"""
import struct
import sys
import zlib


def decode(path):
    data = open(path, 'rb').read()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', 'не PNG'
    pos = 8
    idat = b''
    w = h = depth = ctype = None
    while pos < len(data):
        (ln,) = struct.unpack('>I', data[pos:pos + 4])
        typ = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + ln]
        pos += 12 + ln
        if typ == b'IHDR':
            w, h, depth, ctype = struct.unpack('>IIBB', body[:10])
        elif typ == b'IDAT':
            idat += body
        elif typ == b'IEND':
            break
    assert depth == 8, f'глубина {depth} не поддержана'
    nch = {0: 1, 2: 3, 4: 2, 6: 4}[ctype]
    raw = zlib.decompress(idat)
    stride = w * nch
    out = bytearray(w * h * nch)
    prev = bytearray(stride)
    p = 0
    for y in range(h):
        f = raw[p]
        p += 1
        line = bytearray(raw[p:p + stride])
        p += stride
        if f == 1:
            for i in range(nch, stride):
                line[i] = (line[i] + line[i - nch]) & 0xFF
        elif f == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif f == 3:
            for i in range(stride):
                a = line[i - nch] if i >= nch else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 0xFF
        elif f == 4:
            for i in range(stride):
                a = line[i - nch] if i >= nch else 0
                b = prev[i]
                c = prev[i - nch] if i >= nch else 0
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 0xFF
        out[y * stride:(y + 1) * stride] = line
        prev = line
    return w, h, nch, out


def main(a, b, bx0, by0, bx1, by1):
    wa, ha, na, pa = decode(a)
    wb, hb, nb, pb = decode(b)
    assert (wa, ha) == (wb, hb), f'размеры разные: {wa}x{ha} vs {wb}x{hb}'
    nch = min(na, nb)
    inside = outside = 0
    maxdiff_in = 0
    samples = []
    for y in range(ha):
        for x in range(wa):
            i = (y * wa + x) * nch
            d = max(abs(pa[i + c] - pb[i + c]) for c in range(3))
            if d > 8:
                if bx0 <= x <= bx1 and by0 <= y <= by1:
                    inside += 1
                    maxdiff_in = max(maxdiff_in, d)
                    if len(samples) < 12:
                        samples.append((x, y, tuple(pa[i:i + 3]), tuple(pb[i:i + 3])))
                else:
                    outside += 1
    print(f'кадр {wa}x{ha}')
    print(f'различий внутри bbox спрайта: {inside} (макс. дельта {maxdiff_in})')
    print(f'различий вне bbox: {outside}')
    for s in samples:
        print('  ', s)


if __name__ == '__main__':
    main(*sys.argv[1:2], *sys.argv[2:3], *[int(v) for v in sys.argv[3:7]])
