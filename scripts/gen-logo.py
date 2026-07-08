#!/usr/bin/env python3
"""Генерация логотипа VEK Torrent (assets/logo.png, 1024x1024).

Тёмный скруглённый квадрат с градиентной «V» и стрелкой загрузки.
Из логотипа иконки приложения собираются командой:
    npx tauri icon assets/logo.png -o src-tauri/icons
"""

from __future__ import annotations

import os

from PIL import Image, ImageDraw

S = 1024


def rounded_rect_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    return mask


def linear_gradient(size: int, c1: tuple, c2: tuple) -> Image.Image:
    """Диагональный градиент c1 -> c2."""
    grad = Image.new("RGB", (size, size))
    px = grad.load()
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2 * (size - 1))
            px[x, y] = tuple(int(a + (b - a) * t) for a, b in zip(c1, c2))
    return grad


def main() -> None:
    bg = Image.new("RGB", (S, S), (13, 15, 20))

    # Лёгкое градиентное свечение фона.
    glow = linear_gradient(S, (18, 20, 28), (24, 22, 40))
    bg = Image.blend(bg, glow, 0.85)

    grad = linear_gradient(S, (124, 92, 255), (77, 163, 255))

    # «V» — две толстые диагонали, сходящиеся к нижней точке.
    v_mask = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(v_mask)
    w = 118  # полутолщина штриха
    cx, top, bot = S // 2, 250, 700
    spread = 240
    d.polygon(
        [(cx - spread - w, top), (cx - spread + w, top), (cx + w, bot), (cx - w, bot)],
        fill=255,
    )
    d.polygon(
        [(cx + spread - w, top), (cx + spread + w, top), (cx + w, bot), (cx - w, bot)],
        fill=255,
    )
    # Стрелка загрузки под «V».
    d.polygon([(cx - 150, 760), (cx + 150, 760), (cx, 905)], fill=255)

    logo = bg.copy()
    logo.paste(grad, (0, 0), v_mask)

    out = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    out.paste(logo, (0, 0), rounded_rect_mask(S, 190))

    os.makedirs("assets", exist_ok=True)
    out.save("assets/logo.png")
    print("assets/logo.png готов")


if __name__ == "__main__":
    main()
