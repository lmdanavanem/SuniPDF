#!/usr/bin/env python3
"""Reproduce and verify the portable icon font without additional Python packages."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "assets/fonts"
REVISION = "463d58b8186f874ab2f51cf0a08b4ff5097c912f"
UPSTREAM = "https://github.com/microsoft/fluentui-system-icons"
SOURCE_HASH = "17e14e952734fa82431212cc16841571d35de9ad787091e099e505e72a42ea14"
MAP_HASH = "c63246542a591de8b79a2fa3d54002d74eca68d7352808af387369ed28596aab"
LICENSE_HASH = "69bc45dc42b9acb96a69823adbc6ae538374e3c0bde169b855b32c48eaaef52f"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def tables(data):
    if data[:4] != b"\x00\x01\x00\x00":
        raise ValueError("Expected a TrueType font")
    result = {}
    for index in range(struct.unpack_from(">H", data, 4)[0]):
        tag, _, offset, length = struct.unpack_from(">4sIII", data, 12 + index * 16)
        if tag in result or offset + length > len(data):
            raise ValueError("Invalid TrueType table directory")
        result[tag] = data[offset:offset + length]
    return result


def glyph_map(data):
    table = tables(data)[b"cmap"]
    result = {}
    for index in range(struct.unpack_from(">H", table, 2)[0]):
        platform, encoding, offset = struct.unpack_from(">HHI", table, 4 + index * 8)
        if platform != 0 and (platform, encoding) not in ((3, 1), (3, 10)):
            continue
        kind = struct.unpack_from(">H", table, offset)[0]
        if kind == 12:
            for group in range(struct.unpack_from(">I", table, offset + 12)[0]):
                start, end, glyph = struct.unpack_from(">III", table, offset + 16 + group * 12)
                if end < start or end > 0x10FFFF:
                    raise ValueError("Invalid font character range")
                result.update((code, glyph + code - start) for code in range(start, end + 1) if glyph + code - start)
        elif kind == 4:
            count = struct.unpack_from(">H", table, offset + 6)[0] // 2
            ends = offset + 14
            starts = ends + 2 * count + 2
            deltas = starts + 2 * count
            ranges = deltas + 2 * count
            for segment in range(count):
                start = struct.unpack_from(">H", table, starts + 2 * segment)[0]
                end = struct.unpack_from(">H", table, ends + 2 * segment)[0]
                delta = struct.unpack_from(">H", table, deltas + 2 * segment)[0]
                distance = struct.unpack_from(">H", table, ranges + 2 * segment)[0]
                for code in range(start, end + 1):
                    glyph = struct.unpack_from(">H", table, ranges + 2 * segment + distance + 2 * (code - start))[0] if distance else code
                    glyph = (glyph + delta) & 0xFFFF if glyph or not distance else 0
                    if glyph:
                        result[code] = glyph
    return result


def checksum(data):
    data += bytes((-len(data)) % 4)
    return sum(struct.unpack(">" + "I" * (len(data) // 4), data)) & 0xFFFFFFFF


def remap(data, mapping):
    items = sorted(mapping.items()) + [(0xFFFF, 0)]
    count = len(items)
    power = 1 << (count.bit_length() - 1)
    header = struct.pack(">7H", 4, 16 + count * 8, 0, count * 2, power * 2, power.bit_length() - 1, count * 2 - power * 2)
    codes = struct.pack(">" + "H" * count, *(code for code, _ in items))
    deltas = struct.pack(">" + "H" * count, *((glyph - code) & 0xFFFF for code, glyph in items))
    cmap = struct.pack(">HHHHIHHI", 0, 2, 0, 3, 20, 3, 1, 20) + header + codes + bytes(2) + codes + deltas + bytes(2 * count)
    records = tables(data)
    records[b"cmap"] = cmap
    records.pop(b"DSIG", None)
    records[b"head"] = records[b"head"][:8] + bytes(4) + records[b"head"][12:]
    count = len(records)
    power = 1 << (count.bit_length() - 1)
    output = bytearray(struct.pack(">IHHHH", 0x10000, count, power * 16, power.bit_length() - 1, count * 16 - power * 16) + bytes(16 * count))
    head = None
    for index, (tag, payload) in enumerate(sorted(records.items())):
        offset = len(output)
        struct.pack_into(">4sIII", output, 12 + index * 16, tag, checksum(payload), offset, len(payload))
        output.extend(payload + bytes((-len(payload)) % 4))
        if tag == b"head":
            head = offset
    struct.pack_into(">I", output, head + 8, (0xB1B0AFBA - checksum(bytes(output))) & 0xFFFFFFFF)
    return bytes(output)


def verify():
    provenance = json.loads((DIRECTORY / "icon-provenance.json").read_text())
    font = (DIRECTORY / "SuniPDF-Icons.ttf").read_bytes()
    mapping = json.loads((DIRECTORY / "icon-map.json").read_text())["mapping"]
    if provenance["upstreamRevision"] != REVISION or provenance["upstreamFontSha256"] != SOURCE_HASH:
        raise ValueError("Unexpected upstream icon font provenance")
    for name, expected in (("SuniPDF-Icons.ttf", provenance["sha256"]), ("icon-map.json", provenance["mappingSha256"]), ("FluentSystemIcons-MIT.txt", LICENSE_HASH)):
        if sha((DIRECTORY / name).read_bytes()) != expected:
            raise ValueError("Icon font resource checksum differs: " + name)
    codes = {int(item["codepoint"], 16) for item in mapping}
    used = set()
    for source in (ROOT / "apps/sunipdf-desktop/src").glob("*.rs"):
        used.update(int(code, 16) for code in re.findall(r'"\\u\{([Ee][0-9A-Fa-f]{3})\}"', source.read_text(encoding="utf-8")))
    if used != codes or set(glyph_map(font)) != codes or checksum(font) != 0xB1B0AFBA:
        raise ValueError("Portable font does not cover the actual application icon set")
    return len(codes)


def generate(source_directory):
    original = (source_directory / "FluentSystemIcons-Regular.ttf").read_bytes()
    upstream_map = (source_directory / "FluentSystemIcons-Regular.json").read_bytes()
    if sha(original) != SOURCE_HASH or sha(upstream_map) != MAP_HASH:
        raise ValueError("Upstream icon sources differ from the pinned revision")
    source_mapping = json.loads(upstream_map)
    glyphs = glyph_map(original)
    mapping = json.loads((DIRECTORY / "icon-map.json").read_text())["mapping"]
    remapped = {}
    for item in mapping:
        if source_mapping[item["upstreamName"]] != item["upstreamCodepoint"]:
            raise ValueError("Icon name and upstream codepoint differ")
        remapped[int(item["codepoint"], 16)] = glyphs[item["upstreamCodepoint"]]
    output = remap(original, remapped)
    (DIRECTORY / "SuniPDF-Icons.ttf").write_bytes(output)
    provenance = {"schema": 1, "name": "Fluent System Icons Regular", "license": "MIT", "source": UPSTREAM, "upstreamRevision": REVISION, "upstreamFontSha256": SOURCE_HASH, "upstreamMappingSha256": MAP_HASH, "upstreamFontPath": "fonts/FluentSystemIcons-Regular.ttf", "sha256": sha(output), "mappingSha256": sha((DIRECTORY / "icon-map.json").read_bytes()), "licenseFile": "FluentSystemIcons-MIT.txt", "licenseSha256": LICENSE_HASH, "modification": "The Unicode character map is replaced with the application's 80 icon codepoints. Original outlines, metrics and naming tables are unchanged; this is not a Segoe font. Reproduce with scripts/prepare-icon-font.py and the two pinned upstream source files."}
    (DIRECTORY / "icon-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")
    return verify()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, help="Regenerate from the two pinned upstream font and JSON files; otherwise verify the committed font")
    arguments = parser.parse_args()
    count = generate(arguments.source_dir) if arguments.source_dir else verify()
    print(f"Verified portable icon font: {count} application codepoints")
