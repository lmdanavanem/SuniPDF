"""Génère l'icône originale SuniPDF (Pillow) : assets/icon/sunipdf.ico + PNG 16…512.

Dessin : carré arrondi bleu nuit → bleu, feuille blanche au coin replié, bande cyan en « S » stylisé.
Aucune ressource tierce. Usage : python -I scripts/make_icon.py assets/icon
"""
import os
import sys

from PIL import Image, ImageDraw

NAVY = (17, 24, 39, 255)
BLUE = (37, 99, 235, 255)
CYAN = (6, 182, 212, 255)
WHITE = (255, 255, 255, 255)
PAPER = (248, 250, 252, 255)


def draw_icon(size: int) -> Image.Image:
    s = size * 8  # supersampling ×8 pour un anticrénelage propre
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    r = s * 0.22
    # Fond : dégradé vertical navy → bleu, masqué par un carré arrondi.
    grad = Image.new("RGBA", (s, s))
    gd = ImageDraw.Draw(grad)
    for y in range(s):
        t = y / s
        c = tuple(int(NAVY[i] * (1 - t) + BLUE[i] * t) for i in range(3)) + (255,)
        gd.line([(0, y), (s, y)], fill=c)
    mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, s - 1, s - 1], radius=r, fill=255)
    img.paste(grad, (0, 0), mask)
    # Feuille blanche avec coin replié.
    m = s * 0.22
    left, top, right, bottom = m, m * 0.8, s - m, s - m * 0.8
    fold = (right - left) * 0.28
    d.polygon(
        [(left, top), (right - fold, top), (right, top + fold), (right, bottom), (left, bottom)],
        fill=PAPER,
    )
    d.polygon([(right - fold, top), (right - fold, top + fold), (right, top + fold)], fill=(203, 213, 225, 255))
    # « S » stylisé : deux arcs cyan épais.
    w = int(s * 0.075)
    cx = (left + right) / 2
    cy = (top + bottom) / 2 + s * 0.02
    rad = (right - left) * 0.19
    d.arc([cx - rad, cy - 2 * rad, cx + rad, cy], start=20, end=270, fill=CYAN, width=w)
    d.arc([cx - rad, cy, cx + rad, cy + 2 * rad], start=200, end=90, fill=CYAN, width=w)
    return img.resize((size, size), Image.LANCZOS)


def main(out_dir: str) -> None:
    os.makedirs(out_dir, exist_ok=True)
    sizes = [16, 24, 32, 48, 64, 128, 256, 512]
    images = {n: draw_icon(n) for n in sizes}
    for n, im in images.items():
        im.save(os.path.join(out_dir, f"sunipdf-{n}.png"))
    base = images[256]
    base.save(
        os.path.join(out_dir, "sunipdf.ico"),
        format="ICO",
        sizes=[(n, n) for n in sizes if n <= 256],
        append_images=[images[n] for n in sizes if n <= 256 and n != 256],
    )
    print("ok", os.path.join(out_dir, "sunipdf.ico"))


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "assets/icon")
