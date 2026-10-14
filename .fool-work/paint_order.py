#!/usr/bin/env python3
"""Решающая проверка порядка отрисовки: canvas поверх текста или под ним.

Сравниваем два кадра одной и той же страницы:
  A — canvas как в проде (z-index: 2)
  B — canvas принудительно z-index: -999 (под контентом)
Различия могут быть только там, где сетка и контент перекрываются. Печатаем
цвета: если в A пиксель «испорчен» сеткой поверх белого глифа, а в B он
чисто белый — сетка рисуется поверх текста.
"""
import sys

sys.path.insert(0, __file__.rsplit('/', 1)[0])
from pngdiff import decode  # noqa: E402

CLIP_X, CLIP_Y = 545, 60
EYEBROW = (718, 96, 802, 136)   # «</>» в координатах страницы
TERMINAL = (160, 200, 670, 480)

a, b = sys.argv[1], sys.argv[2]
wa, ha, na, pa = decode(a)
wb, hb, nb, pb = decode(b)

def region_of(px, py):
    if EYEBROW[0] <= px <= EYEBROW[2] and EYEBROW[1] <= py <= EYEBROW[3]:
        return '«</>»'
    if TERMINAL[0] <= px <= TERMINAL[2] and TERMINAL[1] <= py <= TERMINAL[3]:
        return 'терминал'
    return None

print('--- пиксели «</>», где кадры различаются ---')
n = 0
for y in range(ha):
    for x in range(wa):
        i = (y * wa + x) * na
        if max(abs(pa[i + c] - pb[i + c]) for c in range(3)) <= 8:
            continue
        if region_of(x + CLIP_X, y + CLIP_Y) == '«</>»':
            n += 1
            if n <= 10:
                print(f'  страница ({x + CLIP_X},{y + CLIP_Y})  canvas z=2: {tuple(pa[i:i+3])}   canvas z=-999: {tuple(pb[i:i+3])}')
print('всего в «</>»:', n)

print('--- самые светлые различия в «</>» (по кадру B) ---')
rows = []
for y in range(ha):
    for x in range(wa):
        i = (y * wa + x) * na
        if max(abs(pa[i + c] - pb[i + c]) for c in range(3)) <= 8:
            continue
        if region_of(x + CLIP_X, y + CLIP_Y) == '«</>»':
            rows.append((sum(pb[i:i + 3]), x + CLIP_X, y + CLIP_Y, tuple(pa[i:i + 3]), tuple(pb[i:i + 3])))
rows.sort(reverse=True)
for r in rows[:5]:
    print(f'  страница ({r[1]},{r[2]})  A={r[3]}  B={r[4]}')
