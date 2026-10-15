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

def annotated(path):
    """Annotations de plusieurs producteurs (indices 0..11 de la page 1, dans cet ordre) :
    0 Square (Acrobat, /AP, /T Alice) · 1 Ink (/AP) · 2 FreeText (/AP) · 3 Highlight (sans /AP) ·
    4 Text note (/Popup 19) · 5 Text réponse (IRT 4, /T Bob, UTF-16BE) · 6 Text état (IRT 4, Review/Accepted) ·
    7 Link URI · 8 Square sans /AP (entre deux liens) · 9 Link /Dest page 2 · 10 Link Launch · 11 Popup (parent 4).
    Page 2 : 792x612. Aucune annotation n'a de /NM SuniPDF."""
    reply = "Réponse de Bob".encode("utf-16-be").hex().upper().encode()
    c1 = b"BT /F1 24 Tf 72 700 Td (SuniPDF page 1) Tj ET\n"
    c2 = b"BT /F1 24 Tf 72 400 Td (SuniPDF page 2) Tj ET\n"
    def form(bbox, content, font=False):
        res = b"/Resources << /Font << /Helv 7 0 R >> >>" if font else b""
        return (b"<< /Type /XObject /Subtype /Form /BBox [" + bbox + b"] " + res + b" /Length "
                + str(len(content)).encode() + b" >>\nstream\n" + content + b"\nendstream")
    annots = [
        b"<< /Type /Annot /Subtype /Square /Rect [100 500 200 560] /C [1 0 0] /T (Alice) /Contents (Cadre rouge) /F 4 /AP << /N 20 0 R >> >>",
        b"<< /Type /Annot /Subtype /Ink /Rect [105 395 195 455] /InkList [[110 400 150 450 190 400]] /C [0 0 1] /F 4 /AP << /N 21 0 R >> >>",
        b"<< /Type /Annot /Subtype /FreeText /Rect [300 400 460 440] /DA (/Helv 12 Tf 0 0 0 rg) /Contents (Texte libre) /F 4 /AP << /N 22 0 R >> >>",
        b"<< /Type /Annot /Subtype /Highlight /Rect [72 694 250 724] /QuadPoints [72 724 250 724 72 694 250 694] /C [1 1 0] /F 4 >>",
        b"<< /Type /Annot /Subtype /Text /Rect [400 700 420 720] /Contents (Note parent) /T (Alice) /Name /Note /C [1 0.8 0] /F 4 /Popup 19 0 R >>",
        b"<< /Type /Annot /Subtype /Text /Rect [400 700 420 720] /IRT 12 0 R /RT /R /T (Bob) /Contents <FEFF" + reply + b"> /F 4 /AP << /N 23 0 R >> >>",
        b"<< /Type /Annot /Subtype /Text /Rect [400 700 420 720] /IRT 12 0 R /RT /R /T (Carol) /StateModel (Review) /State (Accepted) /F 4 /AP << /N 23 0 R >> >>",
        b"<< /Type /Annot /Subtype /Link /Rect [72 700 300 730] /Border [0 0 0] /A << /S /URI /URI (https://example.com) >> >>",
        b"<< /Type /Annot /Subtype /Square /Rect [250 500 300 540] /C [0 0.6 0] /F 4 >>",
        b"<< /Type /Annot /Subtype /Link /Rect [100 300 200 330] /Border [0 0 0] /Dest [5 0 R /Fit] >>",
        b"<< /Type /Annot /Subtype /Link /Rect [100 250 200 280] /Border [0 0 0] /A << /S /Launch /F (calc.exe) >> >>",
        b"<< /Type /Annot /Subtype /Popup /Rect [450 600 550 700] /Parent 12 0 R /Open false >>",
    ]
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 7 0 R >> >> /Annots ["
        + b" ".join(f"{8 + i} 0 R".encode() for i in range(12)) + b"] >>",
        stream(c1),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 792 612] /Contents 6 0 R /Resources << /Font << /F1 7 0 R >> >> >>",
        stream(c2),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    ] + annots + [
        form(b"100 500 200 560", b"1 0 0 RG 2 w 101 501 98 58 re S"),
        form(b"105 395 195 455", b"0 0 1 RG 2 w 110 400 m 150 450 l 190 400 l S"),
        form(b"300 400 460 440", b"BT /Helv 12 Tf 0 0 0 rg 302 420 Td (Texte libre) Tj ET", font=True),
        form(b"0 0 1 1", b""),
    ]
    open(path, "wb").write(build(objs))

def many_pages(path, n=120):
    """Document long pour la vue continue : Lettre, paysage (i % 10 == 9), A5 (i % 25 == 24),
    carré de couleur ((i*37)%256, (i*91)%256, (i*53)%256) en x 36..156, y 36..116 pt, texte « Page N »."""
    objs = [b"<< /Type /Catalog /Pages 2 0 R >>", None,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"]
    kids = []
    for i in range(n):
        w, h = (792, 612) if i % 10 == 9 else ((420, 595) if i % 25 == 24 else (612, 792))
        r, g, b = (i * 37) % 256, (i * 91) % 256, (i * 53) % 256
        c = (f"{r/255:.4f} {g/255:.4f} {b/255:.4f} rg 36 36 120 80 re f\n0 g\n"
             f"BT /F1 24 Tf 36 {h-60} Td (Page {i+1}) Tj ET\n")
        kids.append(f"{4+2*i} 0 R")
        objs.append(f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Contents {5+2*i} 0 R "
                    f"/Resources << /Font << /F1 3 0 R >> >> >>".encode())
        objs.append(stream(c.encode()))
    objs[1] = f"<< /Type /Pages /Kids [{' '.join(kids)}] /Count {n} >>".encode()
    open(path, "wb").write(build(objs))

def acroform(path, with_ap=True, xfa=False, links=False):
    """Formulaire AcroForm d'une page (612x792) : texte, multiligne, lecture seule, case à cocher, groupe de
    boutons radio, liste déroulante (export/affichage), liste à choix multiple, champs hiérarchiques,
    champ à deux widgets, signature, bouton. with_ap=False : aucun flux d'apparence pour les champs texte/choix/case
    (PDFium les génère). xfa=True : ajoute /XFA à l'AcroForm. links=True : deuxième page (vierge), un lien URI
    (/F 4) et un lien GoTo vers la page 2 (/F 0, action indirecte) sur la page 1, et un champ texte mot de passe
    (/Ff 8192) : l'aplatissement doit conserver les liens, le mot de passe n'est jamais rempli ni exporté."""
    objs = []

    def add(body):
        objs.append(body)
        return len(objs)  # numéro d'objet (1-based)

    def ref(n):
        return b"%d 0 R" % n

    def form_xobject(content, w, h):
        return (b"<< /Type /XObject /Subtype /Form /BBox [0 0 %d %d] /Resources << /Font << /Helv 7 0 R /ZaDb 8 0 R >> >> /Length %d >>\nstream\n"
                % (w, h, len(content))) + content + b"\nendstream"

    def ap_text(w, h, text=b""):
        body = (b"/Tx BMC q BT /Helv 12 Tf 0 g 2 4 Td (%s) Tj ET Q EMC" % text) if text else b"/Tx BMC EMC"
        return form_xobject(body, w, h)

    add(b"")  # 1 catalogue (rempli à la fin)
    add(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>")  # 2
    add(b"")  # 3 page (remplie à la fin)
    labels = [(700, "Nom"), (670, "Ville (accents)"), (640, "Commentaire"), (560, "Reference (lecture seule)"),
              (530, "Accepte les conditions"), (500, "Paiement"), (440, "Pays"), (380, "Langues"),
              (300, "Rue"), (270, "Code postal"), (240, "Telephone"), (200, "Signature")]
    content = b"".join(b"BT /F1 11 Tf 40 %d Td (%s) Tj ET\n" % (y + 4, t.encode("latin-1")) for y, t in labels)
    add(stream(content))  # 4
    add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")  # 5 police des libellés
    add(b"")  # 6 AcroForm (rempli à la fin)
    add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")  # 7 /Helv
    add(b"<< /Type /Font /Subtype /Type1 /BaseFont /ZapfDingbats >>")  # 8 /ZaDb
    fields, annots = [], []

    def new_widget(rect, extra, ap=None, flags=4):
        x1, y1, x2, y2 = rect
        body = b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F %d /Rect [%d %d %d %d] %s" % (flags, x1, y1, x2, y2, extra)
        if ap is not None and with_ap:
            body += b" /AP << /N %s >>" % ap
        n = add(body + b" >>")
        annots.append(n)
        return n

    def apref(w, h, text=b""):
        return ref(add(ap_text(w, h, text))) if with_ap else None

    def apdict(w, h):
        return b"/AP << /N %s >>" % ref(add(ap_text(w, h))) if with_ap else b""

    BOX = b"/MK << /BC [0 0 0] /BG [0.95 0.95 1] >> /DA (/Helv 12 Tf 0 g)"
    # --- champs texte fusionnés avec leur widget ---
    fields.append(new_widget((150, 696, 410, 716), b"/FT /Tx /T (nom) /TU (Nom et prenom) " + BOX, apref(260, 20)))
    # « ville » : widget SANS indicateur d'impression (/F 0) -> piège de l'aplatissement PDFium (mode impression)
    fields.append(new_widget((150, 666, 410, 686), b"/FT /Tx /T (ville) /V (\\311lodie M\\374ller) " + BOX,
                             apref(260, 20, b"\\311lodie M\\374ller"), flags=0))
    fields.append(new_widget((150, 590, 510, 650), b"/FT /Tx /Ff 4096 /T (commentaire) " + BOX, apref(360, 60)))
    fields.append(new_widget((200, 556, 410, 576), b"/FT /Tx /Ff 1 /T (ref) /V (REF-42) " + BOX, apref(210, 20, b"REF-42")))
    # --- case à cocher : AP/N = << /Yes /Off >> ---
    on = add(form_xobject(b"q 0 g 2 2 m 12 12 l 12 2 m 2 12 l S Q", 14, 14))
    off = add(form_xobject(b"", 14, 14))
    fields.append(new_widget((200, 526, 214, 540),
                             b"/FT /Btn /T (accepte) /V /Off /AS /Off /MK << /BC [0 0 0] /CA (8) >> /DA (/ZaDb 0 Tf 0 g)",
                             b"<< /Yes %s /Off %s >>" % (ref(on), ref(off))))
    # --- groupe de boutons radio : champ parent (V = état actif du bouton choisi) + 3 widgets ---
    parent = add(b"")
    kids = []
    for name, y in [("carte", 496), ("virement", 476), ("especes", 456)]:
        a_on = add(form_xobject(b"q 0 g 3 3 8 8 re f Q", 14, 14))
        a_off = add(form_xobject(b"", 14, 14))
        k = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /Rect [200 %d 214 %d] /AS %s /MK << /BC [0 0 0] /CA (l) >> /DA (/ZaDb 0 Tf 0 g) /AP << /N << /%s %s /Off %s >> >> >>"
                % (ref(parent), y, y + 14, b"/virement" if name == "virement" else b"/Off", name.encode(), ref(a_on), ref(a_off)))
        kids.append(k)
        annots.append(k)
    objs[parent - 1] = b"<< /FT /Btn /Ff 49152 /T (paiement) /V /virement /Kids [%s] >>" % b" ".join(ref(k) for k in kids)
    fields.append(parent)
    # --- liste déroulante : /Opt en paires [export affichage] ---
    fields.append(new_widget((150, 436, 350, 456),
                             b"/FT /Ch /Ff 131072 /T (pays) /Opt [[(FR) (France)] [(BE) (Belgique)] [(CA) (Canada)]] /V (BE) " + BOX,
                             apref(200, 20, b"Belgique")))
    # --- liste à choix multiple ---
    fields.append(new_widget((150, 350, 350, 410),
                             b"/FT /Ch /Ff 2097152 /T (langues) /Opt [(Fran\\347ais) (Anglais) (Allemand)] /V [(Anglais)] /I [1] " + BOX,
                             apref(200, 60)))
    # --- champs hiérarchiques adresse.rue et adresse.code (MaxLen 5) ---
    addr = add(b"")
    k1 = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /FT /Tx /T (rue) /Rect [150 296 410 316] %s %s >>"
             % (ref(addr), BOX, apdict(260, 20)))
    k2 = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /FT /Tx /T (code) /MaxLen 5 /Rect [150 266 250 286] %s %s >>"
             % (ref(addr), BOX, apdict(100, 20)))
    objs[addr - 1] = b"<< /T (adresse) /Kids [%s %s] >>" % (ref(k1), ref(k2))
    annots += [k1, k2]
    fields.append(addr)
    # --- un champ à deux widgets (même valeur affichée deux fois) ---
    tel = add(b"")
    w1 = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /Rect [150 236 300 256] %s %s >>" % (ref(tel), BOX, apdict(150, 20)))
    w2 = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /Rect [320 236 470 256] %s %s >>" % (ref(tel), BOX, apdict(150, 20)))
    objs[tel - 1] = b"<< /FT /Tx /T (telephone) /Kids [%s %s] >>" % (ref(w1), ref(w2))
    annots += [w1, w2]
    fields.append(tel)
    # --- signature (non remplissable ici) et bouton-poussoir ---
    fields.append(new_widget((150, 190, 350, 230), b"/FT /Sig /T (signature)"))
    fields.append(new_widget((400, 190, 500, 214),
                             b"/FT /Btn /Ff 65536 /T (envoyer) /MK << /BC [0 0 0] /BG [0.8 0.8 0.8] /CA (Envoyer) >> /DA (/Helv 12 Tf 0 g)"))
    if links:
        fields.append(new_widget((150, 150, 350, 170), b"/FT /Tx /Ff 8192 /T (secret) " + BOX, apref(200, 20)))
        page2 = add(b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>")
        objs[1] = b"<< /Type /Pages /Kids [3 0 R %s] /Count 2 >>" % ref(page2)
        annots.append(add(b"<< /Type /Annot /Subtype /Link /F 4 /Rect [40 740 200 760] /Border [0 0 0] "
                          b"/A << /S /URI /URI (https://example.org/suni) >> >>"))
        goto = add(b"<< /S /GoTo /D [%s /Fit] >>" % ref(page2))
        annots.append(add(b"<< /Type /Annot /Subtype /Link /F 0 /Rect [300 740 500 760] /Border [0 0 0] /A %s >>" % ref(goto)))
    acro = b"<< /Fields [%s] /DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv 7 0 R /ZaDb 8 0 R >> >>" % b" ".join(ref(f) for f in fields)
    if xfa:
        acro += b" /XFA [(preamble) 4 0 R]"
    objs[5] = acro + b" >>"
    objs[0] = b"<< /Type /Catalog /Pages 2 0 R /AcroForm 6 0 R >>"
    objs[2] = (b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> /Annots [%s] >>"
               % b" ".join(ref(a) for a in annots))
    open(path, "wb").write(build(objs))

if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--many":
        # Fichier long à la demande (non versionné) : --many 500 %TEMP%\many-500.pdf
        many_pages(sys.argv[3], int(sys.argv[2]))
        print("ok")
        sys.exit(0)
    out = sys.argv[1]
    two_pages(os.path.join(out, "two-pages.pdf"))
    encrypted(os.path.join(out, "encrypted-rc4.pdf"))
    boxes(os.path.join(out, "boxes.pdf"))
    many_pages(os.path.join(out, "many-pages.pdf"), 120)
    annotated(os.path.join(out, "annotated.pdf"))
    acroform(os.path.join(out, "acroform.pdf"))
    acroform(os.path.join(out, "acroform-noap.pdf"), with_ap=False)
    acroform(os.path.join(out, "acroform-xfa.pdf"), xfa=True)
    acroform(os.path.join(out, "acroform-links.pdf"), links=True)
    print("ok")
