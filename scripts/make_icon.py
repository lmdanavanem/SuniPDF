"""Dérive l'icône et les logos SuniPDF depuis les sources transparentes (Pillow).

Sources : assets/logo/sunipdf-logo-light.png (texte sombre, thème clair) et
assets/logo/sunipdf-logo-dark.png (texte clair, thème sombre), 1254 × 1254, fond transparent.
Icône : assets/icon/sunipdf-source.png, symbole plein cadre fourni par l'auteur.
Produits :
  assets/icon/sunipdf.ico + sunipdf-{16,24,32,48,64,128,256,512}.png : le symbole seul (page et ruban),
  pour l'exécutable, la fenêtre, le MSI et la barre des tâches ;
  assets/logo/sunipdf-wordmark-{light,dark}.png : logo complet réduit (512 px) pour l'interface et le README.
Usage : python -I scripts/make_icon.py assets/icon
"""
import os
import sys

from PIL import Image

SIZES = [16, 24, 32, 48, 64, 128, 256, 512]


def main(out_dir: str) -> None:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    logo_dir = os.path.join(root, "assets", "logo")
    light = Image.open(os.path.join(logo_dir, "sunipdf-logo-light.png")).convert("RGBA")
    dark = Image.open(os.path.join(logo_dir, "sunipdf-logo-dark.png")).convert("RGBA")
    os.makedirs(out_dir, exist_ok=True)
    sym = Image.open(os.path.join(root, "assets", "icon", "sunipdf-source.png")).convert("RGBA")
    assert sym.width == sym.height, "Icon source must be square"
    pngs = []
    for size in SIZES:
        im = sym.resize((size, size), Image.LANCZOS)
        box = im.getchannel("A").point(lambda alpha: 255 if alpha >= 32 else 0).getbbox()
        assert box and min(box[2] - box[0], box[3] - box[1]) >= size * 0.88, "Icon has excessive margins"
        assert im.getpixel((0, 0))[3] == 0, "Icon background must be transparent"
        path = os.path.join(out_dir, f"sunipdf-{size}.png")
        im.save(path, optimize=True)
        pngs.append(im)
    pngs[-1].save(
        os.path.join(out_dir, "sunipdf.ico"),
        format="ICO",
        sizes=[(s, s) for s in SIZES if s <= 256],
        append_images=pngs[:-1],
    )
    with Image.open(os.path.join(out_dir, "sunipdf.ico")) as ico:
        assert ico.ico.sizes() == {(s, s) for s in SIZES if s <= 256}, "Missing Windows icon size"
    for name, logo in (("light", light), ("dark", dark)):
        box = logo.split()[3].getbbox()
        cropped = logo.crop(box)
        cropped.thumbnail((512, 512), Image.LANCZOS)
        cropped.save(os.path.join(logo_dir, f"sunipdf-wordmark-{name}.png"), optimize=True)
    print(f"OK : {out_dir} (ico + {len(SIZES)} PNG), logos {logo_dir}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "assets/icon")
