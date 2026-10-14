"""Génère des PDF de test minimalistes (sans dépendance) pour sunipdf-core."""
import hashlib, struct, sys, os

PAD = bytes([0x28,0xBF,0x4E,0x5E,0x4E,0x75,0x8A,0x41,0x64,0x00,0x4E,0x56,0xFF,0xFA,0x01,0x08,
             0x2E,0x2E,0x00,0xB6,0xD0,0x68,0x3E,0x80,0x2F,0x0C,0xA9,0xFE,0x64,0x53,0x69,0x7A])

def rc4(key, data):
    S = list(range(256)); j = 0; out = bytearray()
    for i in range(256):
        j = (j + S[i] + key[i % len(key)]) & 0xFF; S[i], S[j] = S[j], S[i]
    i = j = 0
    for c in data:
        i = (i + 1) & 0xFF; j = (j + S[i]) & 0xFF; S[i], S[j] = S[j], S[i]
        out.append(c ^ S[(S[i] + S[j]) & 0xFF])
    return bytes(out)

def build(objects, trailer_extra=b"", encrypt=None):
    """objects: list of bytes bodies (object i+1). encrypt: (key) -> encrypt streams/strings of obj n."""
    out = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for n, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += f"{n} 0 obj\n".encode() + body + b"\nendobj\n"
    xref = len(out)
    out += f"xref\n0 {len(objects)+1}\n".encode()
    out += b"0000000000 65535 f \n"
    for o in offsets:
        out += f"{o:010d} 00000 n \n".encode()
    out += f"trailer\n<< /Size {len(objects)+1} /Root 1 0 R ".encode() + trailer_extra + b" >>\n"
    out += f"startxref\n{xref}\n%%EOF\n".encode()
    return bytes(out)

def stream(content, enc=None):
    data = enc(content) if enc else content
    return b"<< /Length " + str(len(data)).encode() + b" >>\nstream\n" + data + b"\nendstream"

def two_pages(path):
    c1 = (b"0 0 1 rg 72 500 200 150 re f\n"
          b"BT /F1 24 Tf 72 700 Td (SuniPDF page 1) Tj ET\n")
    c2 = b"1 0 0 rg 72 72 300 100 re f\nBT /F1 24 Tf 72 400 Td (SuniPDF page 2) Tj ET\n"
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R /Outlines 9 0 R /PageMode /UseOutlines >>",
        b"<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 7 0 R >> >> >>",
        stream(c1),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 792 612] /Contents 6 0 R /Resources << /Font << /F1 7 0 R >> >> >>",
        stream(c2),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Title (SuniPDF fixture) /Author (SuniPDF tests) >>",
        b"<< /Type /Outlines /First 10 0 R /Last 11 0 R /Count 3 >>",
        b"<< /Title (Premiere page) /Parent 9 0 R /Next 11 0 R /First 12 0 R /Last 12 0 R /Count 1 /Dest [3 0 R /Fit] >>",
        b"<< /Title (Deuxieme page) /Parent 9 0 R /Prev 10 0 R /Dest [5 0 R /Fit] >>",
        b"<< /Title (Rectangle bleu) /Parent 10 0 R /Dest [3 0 R /XYZ 72 650 0] >>",
    ]
    data = build(objs, trailer_extra=b"/Info 8 0 R")
    open(path, "wb").write(data)

def encrypted(path, user_pw=b"secret"):
    # Standard security handler, R2 / V1, RC4 40 bits, P = -1, owner pw = user pw.
    doc_id = hashlib.md5(b"sunipdf-encrypted-fixture").digest()
    padded_user = (user_pw + PAD)[:32]
    okey = hashlib.md5(padded_user).digest()[:5]           # owner == user
    O = rc4(okey, padded_user)
    P = -1
    key = hashlib.md5(padded_user + O + struct.pack("<i", P) + doc_id).digest()[:5]
    U = rc4(key, PAD)
    def objkey(n, g=0):
        return hashlib.md5(key + struct.pack("<I", n)[:3] + struct.pack("<H", g)).digest()[:10]
    def enc_for(n):
        return lambda data: rc4(objkey(n), data)
    content = b"0 0.5 0 rg 100 100 200 200 re f\nBT /F1 18 Tf 72 700 Td (Encrypted page) Tj ET\n"
    def hexs(b): return b"<" + b.hex().encode() + b">"
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        stream(content, enc_for(4)),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Filter /Standard /V 1 /R 2 /Length 40 /P " + str(P).encode() + b" /O " + hexs(O) + b" /U " + hexs(U) + b" >>",
    ]
    data = build(objs, trailer_extra=b"/Encrypt 6 0 R /ID [" + hexs(doc_id) + b" " + hexs(doc_id) + b"]")
    open(path, "wb").write(data)

def boxes(path):
    """Pages tournées et rognées : carrés rouge et bleu, texte, à des positions utilisateur connues."""
    def c(rx, ry, bx, by, tx, ty, name):
        return (f"1 0 0 rg {rx} {ry} 100 100 re f\n0 0 1 rg {bx} {by} 100 100 re f\n"
                f"0 0 0 rg BT /F1 24 Tf {tx} {ty} Td (BOXES {name}) Tj ET\n").encode()
    pages = [
        (b"/MediaBox [0 0 612 792] /Rotate 90", c(50, 50, 460, 640, 200, 400, "A")),
        (b"/MediaBox [0 0 612 792] /Rotate 180", c(50, 50, 460, 640, 200, 400, "B")),
        (b"/MediaBox [100 50 712 842] /CropBox [150 100 650 700] /Rotate 270",
         c(160, 110, 540, 590, 300, 400, "C")),
        (b"/MediaBox [0 0 612 792] /CropBox [100 100 500 600]", c(110, 110, 390, 490, 200, 300, "D")),
    ]
    kids = " ".join(f"{4 + 2 * i} 0 R" for i in range(len(pages))).encode()
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [" + kids + b"] /Count " + str(len(pages)).encode() + b" >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    for i, (attrs, content) in enumerate(pages):
        objs.append(b"<< /Type /Page /Parent 2 0 R " + attrs + b" /Contents " + str(5 + 2 * i).encode()
                    + b" 0 R /Resources << /Font << /F1 3 0 R >> >> >>")
        objs.append(stream(content))
    open(path, "wb").write(build(objs))

if __name__ == "__main__":
    out = sys.argv[1]
    two_pages(os.path.join(out, "two-pages.pdf"))
    encrypted(os.path.join(out, "encrypted-rc4.pdf"))
    boxes(os.path.join(out, "boxes.pdf"))
    print("ok")
