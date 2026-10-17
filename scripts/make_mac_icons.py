"""Produit assets/fonts/SuniPDF-Icons.ttf pour macOS, où Segoe Fluent Icons n'existe pas.

Chaque point de code Segoe utilisé par apps/sunipdf-desktop/src/icons.rs reçoit un glyphe de
Fluent UI System Icons (MIT, même langage visuel) : la table `glyph` Rust reste inchangée.

Entrées : FluentSystemIcons-Regular.ttf et .json de https://github.com/microsoft/fluentui-system-icons
(commit a347af95478f989038cccbf085978865264c048a).
Usage : python -I scripts/make_mac_icons.py <Regular.ttf> <Regular.json> assets/fonts/SuniPDF-Icons.ttf
(nécessite fonttools)
"""
import json
import sys

from fontTools.subset import Options, Subsetter
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable

# Point de code Segoe -> icône Fluent (20 px, regular). À tenir aligné sur icons.rs `glyph`.
MAP = {
    0xE700: "navigation", 0xE80F: "home", 0xE74E: "save", 0xE792: "save_edit",
    0xE721: "search", 0xE8A3: "zoom_in", 0xE71F: "zoom_out", 0xE7A7: "arrow_undo",
    0xE7A6: "arrow_redo", 0xE74D: "delete", 0xE711: "dismiss", 0xE70E: "chevron_up",
    0xE70D: "chevron_down", 0xE897: "question_circle", 0xE946: "info", 0xE8A5: "document",
    0xE706: "weather_sunny", 0xE708: "weather_moon", 0xE9A6: "zoom_fit",
    0xE740: "arrow_autofit_width", 0xE8A9: "grid", 0xE71D: "apps", 0xE838: "folder_open",
    0xE774: "globe", 0xE7C3: "document_one_page_multiple", 0xE8E6: "text_t",
    0xE7E6: "highlight", 0xE8DC: "text_underline", 0xEDE0: "text_strikethrough",
    0xE70B: "note", 0xE90A: "comment", 0xE8A4: "bookmark", 0xE72E: "lock_closed",
    0xE785: "lock_open", 0xE75C: "eraser", 0xEA3C: "merge", 0xE8C6: "cut",
    0xE8D1: "document_arrow_up", 0xE8B9: "image_add", 0xE92C: "arrow_minimize",
    0xE8B0: "cursor", 0xE8D2: "text_font", 0xE8AC: "rename", 0xE739: "rectangle_landscape",
    0xE73B: "checkbox_indeterminate", 0xEA3A: "oval", 0xE738: "line_horizontal_1",
    0xE8C8: "copy", 0xE80C: "arrow_rotate_counterclockwise",
    0xE80D: "arrow_rotate_clockwise", 0xE74A: "position_to_front",
    0xE74B: "position_to_back", 0xE7BA: "warning", 0xEA18: "shield", 0xE8EC: "tag",
    0xE8EF: "document_page_number", 0xEDE1: "arrow_export_ltr", 0xE70F: "edit",
    0xE710: "add", 0xE76B: "chevron_left", 0xE76C: "chevron_right", 0xE72B: "arrow_left",
    0xE8B5: "arrow_import", 0xE7A8: "crop", 0xE77F: "clipboard_paste",
    0xE8B3: "select_all_on", 0xE81E: "layer", 0xE91B: "image",
    0xECE7: "arrow_bidirectional_up_down", 0xE929: "signature", 0xE787: "calendar",
    0xE9D5: "task_list_ltr", 0xE762: "multiselect_ltr", 0xEC87: "inking_tool",
    0xE8E9: "textbox", 0xE71B: "link", 0xE7C9: "cursor_click",
    # Écrit en dur dans shell.rs (Imprimer).
    0xE749: "print",
}


def main(ttf: str, names_json: str, out: str) -> None:
    with open(names_json, encoding="utf-8") as f:
        names = json.load(f)
    font = TTFont(ttf)
    by_code = font.getBestCmap()
    cmap = {}
    for segoe, name in MAP.items():
        code = names.get(f"ic_fluent_{name}_20_regular")
        if code is None:
            sys.exit(f"icône Fluent absente : {name}")
        cmap[segoe] = by_code[code]

    options = Options()
    options.notdef_outline = True
    options.name_IDs = ["*"]
    subsetter = Subsetter(options)
    subsetter.populate(glyphs=sorted(set(cmap.values())))
    subsetter.subset(font)

    table = CmapSubtable.newSubtable(4)
    table.platformID, table.platEncID, table.language = 3, 1, 0
    table.cmap = cmap
    font["cmap"].tables = [table]
    # Sous-ensemble modifié : il ne se présente pas sous le nom de la police d'origine.
    for record in font["name"].names:
        if record.nameID in (1, 4, 16):
            record.string = "SuniPDF Icons"
        elif record.nameID == 6:
            record.string = "SuniPDF-Icons"
    font.save(out)


if __name__ == "__main__":
    main(*sys.argv[1:4])
