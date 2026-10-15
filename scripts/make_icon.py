"""Dérive l'icône et les logos SuniPDF depuis les logos sources (Pillow).

Sources : assets/logo/sunipdf-logo-light.png (texte sombre, thème clair) et
assets/logo/sunipdf-logo-dark.png (texte clair, thème sombre), 1254 × 1254, fond transparent.
Produits :
  assets/icon/sunipdf.ico + sunipdf-{16,24,32,48,64,128,256,512}.png : le symbole seul (page et ruban),
  recadré au carré, pour l'exécutable, la fenêtre, le MSI et la barre des tâches ;
  assets/logo/sunipdf-wordmark-{light,dark}.png : logo complet réduit (512 px) pour l'interface et le README.
Usage : python -I scripts/make_icon.py assets/icon
"""
import os
import sys

from PIL import Image

SIZES = [16, 24, 32, 48, 64, 128, 256, 512]


def symbol(logo: Image.Image) -> Image.Image:
    """Symbole seul : partie haute du logo (au-dessus du mot), recadrée au carré avec une marge."""
    w, h = logo.size
    top = logo.crop((0, 0, w, int(h * 0.66)))
    box = top.split()[3].getbbox()
    if box is None:
        raise SystemExit("logo vide")
    left, up, right, down = box
    side = max(right - left, down - up)
    margin = int(side * 0.04)
    side += 2 * margin
    cx, cy = (left + right) // 2, (up + down) // 2
    sq = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    sq.paste(top.crop((cx - side // 2, cy - side // 2, cx + side // 2, cy + side // 2)), (0, 0))
    return sq


def main(out_dir: str) -> None:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    logo_dir = os.path.join(root, "assets", "logo")
    light = Image.open(os.path.join(logo_dir, "sunipdf-logo-light.png")).convert("RGBA")
    dark = Image.open(os.path.join(logo_dir, "sunipdf-logo-dark.png")).convert("RGBA")
    os.makedirs(out_dir, exist_ok=True)
    sym = symbol(light)
    pngs = []
    for size in SIZES:
        im = sym.resize((size, size), Image.LANCZOS)
        path = os.path.join(out_dir, f"sunipdf-{size}.png")
        im.save(path, optimize=True)
        pngs.append(im)
    pngs[-1].save(
        os.path.join(out_dir, "sunipdf.ico"),
        format="ICO",
        sizes=[(s, s) for s in SIZES if s <= 256],
        append_images=pngs[:-1],
    )
    for name, logo in (("light", light), ("dark", dark)):
        box = logo.split()[3].getbbox()
        cropped = logo.crop(box)
        cropped.thumbnail((512, 512), Image.LANCZOS)
        cropped.save(os.path.join(logo_dir, f"sunipdf-wordmark-{name}.png"), optimize=True)
    print(f"OK : {out_dir} (ico + {len(SIZES)} PNG), logos {logo_dir}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "assets/icon")
