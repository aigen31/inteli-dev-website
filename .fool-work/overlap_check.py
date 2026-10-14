#!/usr/bin/env python3
"""Проверка: canvas-сетка рисуется поверх контента hero (или под ним).

Сравниваем кадр с canvas и без него. Для каждого различия смотрим, что
именно перекрыто: непрозрачный пиксель спрайта, текст «</>», окно терминала
или просто фон. Плюс печатаем, какую долю спрайта задевает сетка.
"""
import sys

sys.path.insert(0, __file__.rsplit('/', 1)[0])
from pngdiff import decode  # noqa: E402

CLIP_X, CLIP_Y = 545, 60
MASCOT = (593, 103, 676, 225)
EYEBROW = (718, 96, 802, 136)
TERMINAL = (160, 200, 670, 480)

a, b, sprite_path = sys.argv[1], sys.argv[2], sys.argv[3]
wa, ha, na, pa = decode(a)
wb, hb, nb, pb = decode(b)
sw, sh, sn, sp = decode(sprite_path)
alpha_off = 3 if sn == 4 else None

def sprite_alpha(px, py):
    lx, ly = px - MASCOT[0], py - MASCOT[1]
    if not (0 <= lx < sw and 0 <= ly < sh):
        return 0
    if alpha_off is None:
        return 255
    return sp[(ly * sw + lx) * sn + alpha_off]

def inside(box, x, y):
    return box[0] <= x <= box[2] and box[1] <= y <= box[3]

stats = {
    'спрайт (непрозрачные пиксели)': 0,
    'спрайт (прозрачная зона внутри bbox)': 0,
    'терминал': 0,
    '«</>»': 0,
    'прочее (фон hero)': 0,
}
maxd = {'спрайт (непрозрачные пиксели)': 0}
for y in range(ha):
    for x in range(wa):
        i = (y * wa + x) * na
        d = max(abs(pa[i + c] - pb[i + c]) for c in range(3))
        if d <= 8:
            continue
        px, py = x + CLIP_X, y + CLIP_Y
        if inside(MASCOT, px, py):
            key = 'спрайт (непрозрачные пиксели)' if sprite_alpha(px, py) > 0 else 'спрайт (прозрачная зона внутри bbox)'
        elif inside(TERMINAL, px, py):
            key = 'терминал'
        elif inside(EYEBROW, px, py):
            key = '«</>»'
        else:
            key = 'прочее (фон hero)'
        stats[key] += 1
        maxd[key] = max(maxd.get(key, 0), d)

opaque_total = sum(1 for ly in range(sh) for lx in range(sw) if sprite_alpha(lx + MASCOT[0], ly + MASCOT[1]) > 0)
print(f'непрозрачных пикселей в спрайте: {opaque_total}')
for k, v in stats.items():
    print(f'{k}: {v} (макс. дельта {maxd.get(k, 0)})')
