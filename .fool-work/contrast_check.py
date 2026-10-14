#!/usr/bin/env python3
"""Читаемость маскота: контраст тёмных частей спрайта с фоном hero.

Считаем по кадру БЕЗ canvas-сетки (чтобы не мешала), и по самому спрайту:
  — средний фон вокруг спрайта;
  — доля пикселей спрайта, которые темнее фона,
  — контраст по WCAG между фоном и тёмными/светлыми частями спрайта.
"""
import sys

sys.path.insert(0, __file__.rsplit('/', 1)[0])
from pngdiff import decode  # noqa: E402

CLIP_X, CLIP_Y = 545, 60
MASCOT = (593, 103, 676, 225)   # координаты страницы


def luma(r, g, b):
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def rel(r, g, b):
    def ch(v):
        v /= 255.0
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)


def contrast(c1, c2):
    l1, l2 = rel(*c1), rel(*c2)
    hi, lo = max(l1, l2), min(l1, l2)
    return (hi + 0.05) / (lo + 0.05)


frame, sprite_path = sys.argv[1], sys.argv[2]
fw, fh, fn, fp = decode(frame)
sw, sh, sn, sp = decode(sprite_path)

# фон: кольцо вокруг спрайта в кадре
ring = []
for y in range(max(0, MASCOT[1] - 30 - CLIP_Y), min(fh, MASCOT[3] + 30 - CLIP_Y)):
    for x in range(max(0, MASCOT[0] - 30 - CLIP_X), min(fw, MASCOT[2] + 30 - CLIP_X)):
        px, py = x + CLIP_X, y + CLIP_Y
        if MASCOT[0] - 4 <= px <= MASCOT[2] + 4 and MASCOT[1] - 4 <= py <= MASCOT[3] + 4:
            continue
        i = (y * fw + x) * fn
        ring.append((fp[i], fp[i + 1], fp[i + 2]))
bg = tuple(sum(c[k] for c in ring) / len(ring) for k in range(3))
print(f'фон вокруг спрайта: {tuple(round(v) for v in bg)}  (пикселей {len(ring)})')

# пиксели спрайта
opaque = []
for ly in range(sh):
    for lx in range(sw):
        i = (ly * sw + lx) * sn
        if sn == 4 and sp[i + 3] == 0:
            continue
        opaque.append((sp[i], sp[i + 1], sp[i + 2]))
op = [luma(*c) for c in opaque]
bg_l = luma(*bg)
dark = sum(1 for v in op if v < bg_l)
print(f'пикселей в спрайте: {len(opaque)}')
print(f'средняя яркость спрайта: {sum(op)/len(op):.1f}, фон: {bg_l:.1f}')
print(f'темнее фона: {dark} ({100*dark/len(op):.1f}%)')
darker = [c for c in opaque if luma(*c) < bg_l]
if darker:
    mc = sum(contrast(c, bg) for c in darker) / len(darker)
    print(f'средний контраст {len(darker)} пикселей темнее фона: {mc:.2f}')
print(f'контраст самого тёмного пикселя спрайта с фоном: {contrast(opaque[op.index(min(op))], bg):.2f}')
light = opaque[op.index(max(op))]
print(f'контраст самого светлого пикселя спрайта с фоном: {contrast(light, bg):.2f}')
q = sorted(op)
print(f'перцентили яркости: p10={q[len(q)//10]:.0f} p50={q[len(q)//2]:.0f} p90={q[9*len(q)//10]:.0f}')
