# Formats de polices intégrées et sous-ensembles

Le texte ajouté utilise une face sélectionnée, dont le contenu est vérifié. Les
contours TrueType statiques des fichiers TTF/OTF et des faces TTC/OTC sélectionnées
utilisent le chargeur CID TrueType de PDFium. Les faces CFF1 statiques utilisent son
chargeur CID Type1. Les faces CFF1 à clés CID doivent appartenir à la collection
Adobe-Identity. Allsorts réécrit leur charset pour correspondre aux codes Identity-H
de PDFium, y compris pour une face complète sans réduction. Les contours,
sous-programmes, cmap et droits restent intacts. Les autres collections CID sont
refusées avant modification, car elles nécessitent une écriture PDF adaptée.
Les polices variables, CFF2, les tables de couleur, les fichiers
endommagés, les contours absents et les conteneurs web sont refusés. Un format ne
doit être annoncé comme validé qu'après réussite de ses tests natifs et des contrôles
par un lecteur indépendant.

Allsorts 0.17.0, avec son moteur flate2 en Rust, produit les sous-ensembles. Cette
bibliothèque conserve un cmap Unicode et les dépendances des glyphes composites ;
SuniPDF ne réécrit pas les contours avec un algorithme interne. Le profil Minimal
conserve OS/2 et les tables SFNT nécessaires. La disponibilité des caractères, les
droits et les limites sont contrôlés avant la modification de la page.
`TextStyle.font_file` désigne le fichier ; `font_face_index` sélectionne sa face à
partir de zéro. Le sélecteur distingue les faces d'une collection.

Les droits OS/2.fsType de la face sélectionnée sont vérifiés avant toute réduction.
L'intégration restreinte, limitée à l'aperçu ou à l'impression, et limitée aux bitmaps
est refusée. Le bit interdisant les sous-ensembles impose l'intégration de la face
complète, y compris lorsqu'elle doit être extraite d'une collection. Les bits réservés
sont refusés. L'interprétation existante des anciennes combinaisons de droits garde
le droit le moins restrictif. Ces indicateurs ne remplacent pas les conditions de
licence du titulaire de la police.

Le sous-ensemble comprend les glyphes demandés, leurs dépendances et les caractères
présents dans les plages U+0020–U+024F et U+0370–U+052F. Cette réserve permet les
modifications courantes en latin, grec et cyrillique. Un sous-ensemble existant ne
reçoit pas automatiquement de nouveaux caractères : une modification incompatible
renvoie MissingGlyphs et conserve le document. L'outil Ajouter du texte peut réutiliser
la police source pour un autre ensemble de caractères. Une face autonome reste
complète si sa réduction serait plus volumineuse.

Les entrées sont limitées à 32 Mio, 64 faces, 65 536 caractères et 4 096 glyphes
sélectionnés pour une réduction. La lecture applique une limite d'octets en plus du
contrôle de taille du fichier. Les programmes préparés sont comparés octet par octet
pour réutiliser une police déjà intégrée. Après Annuler, Rétablir ou une réouverture,
les polices des objets texte existants sont examinées. Les handles natifs de l'ancien
document sont toujours invalidés après restauration.

Les fichiers de régression conservent leurs sources, empreintes, licences complètes
et mentions d'origine dans `crates/sunipdf-core/tests/fixtures/fonts`. Les contrôles
couvrent Unicode, réduction effective, interdiction de réduction, sélection de faces,
refus et réutilisation après sauvegarde/réouverture. Les tests du moteur produisent
quatre PDF, leurs images de référence obtenues avec les polices complètes et leurs
métadonnées dans `target/font-embedding-evidence`. Le lecteur MuPDF de test vérifie
les caractères extraits et la géométrie des glyphes à 144 dpi. La création des fichiers
ou la réussite des tests PDFium ne suffit pas à prouver la compatibilité avec un autre
lecteur. Les résultats réels doivent accompagner la validation de la version.

Références : [droits d'intégration Microsoft](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fstype),
[conteneurs OpenType](https://learn.microsoft.com/en-us/typography/opentype/spec/otff),
[sources Allsorts](https://github.com/yeslogic/allsorts),
[API de polices PDFium](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/fpdf_edit.h).
