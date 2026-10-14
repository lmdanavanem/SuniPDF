# Matrice fonctionnelle PDFCraft / SuniPDF

Statuts PDFCraft d'après son README et ROADMAP (2026-10-13) : **Déclaré** = annoncé par le projet,
**Partiel** = annoncé comme incomplet par le projet, **Absent** = annoncé manquant. Non vérifié par exécution.

Statuts SuniPDF : **Fait** = implémenté **et** couvert par des tests exécutés ; **En cours** ; **Planifié (Pn)** = phase
de la roadmap ; **Hors périmètre**. Mettre à jour cette colonne uniquement avec des résultats réels.

## A. Lecture

| Fonction | PDFCraft | SuniPDF | Note |
|---|---|---|---|
| Ouvrir un PDF, afficher une page | Déclaré | **Fait** | PDFium ; tests `crates/sunipdf-core/tests/engine.rs` |
| Documents protégés par mot de passe | Déclaré (toutes révisions) | **Fait** | RC4 40 bits testé ; AES par PDFium, à couvrir par fixture (P2) |
| Navigation page/précédente/suivante/aller à | Déclaré | **Fait** (UI) | Clavier + barre d'outils |
| Zoom, ajuster largeur/page, taille réelle | Déclaré | **Fait** (UI) | Rendu à la résolution physique (DPI) ; Ctrl + molette ancré sous le pointeur, 10 à 3200 % |
| Rendu hors thread UI | Déclaré | **Fait** | Thread moteur unique, liste de tuiles voulues remplacée en bloc, époques anti-obsolescence |
| Onglets multi-documents | Déclaré | **Fait** (suspension des documents inactifs, état d'interface conservé) | |
| Affichage continu, double page, rotation de vue | Déclaré | Affichage continu **Fait** (virtualisé, page courante suivie, page unique en option F5) ; double page et rotation de vue planifiées | Tests `view.rs` (mise en page, virtualisation, ancre de zoom), captures |
| Rendu en tuiles, cache plafonné | Déclaré | **Fait** (tuiles de 1024 px au-delà de 4096 px / 8 Mpx, cache LRU 256 Mo, textures 192 Mo) | `Document::render_region`, tests de pixels (régions, recollage, zoom ×40) ; CLI `render --region` |
| Recherche plein texte | Déclaré | **Fait** | Ctrl+F, casse, surbrillance, CLI `search` |
| Sélection / copie de texte en ordre de lecture | Déclaré | **Fait** (sélection rectangulaire, lignes fusionnées) | ordre de lecture complexe planifié |
| Miniatures | Déclaré | **Fait** | Panneau virtualisé, cache partagé |
| Signets (outline) | Déclaré | **Fait** (panneau, navigation ; édition planifiée) | |
| Étiquettes de pages | Déclaré | Planifié (P3) | |
| Historique de navigation, onglets rouverts | Déclaré | Fichiers récents **Faits** (accueil) ; historique de navigation planifié (P3) | |
| Mode lecture / plein écran | Déclaré | Planifié (P3) | |
| Impression Windows | Déclaré (fidélité limitée) | Planifié (P5) | API Windows (`sunipdf-platform`) |
| Récupération après plantage, autosave | Déclaré | Planifié (P3) | |
| Thèmes clair/sombre | Déclaré | **Fait** | Design system `theme.rs`, bascule dans la barre d'onglets |
| Interface traduisible | Déclaré (EN, JA + 22 catalogues) | **Fait** (fr-FR défaut, en-US) | Changement à chaud, préférence persistée |

## B. Édition et annotation

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| Notes, surlignage, souligné, barré, formes, encre, zones de texte | Déclaré | Notes, surlignage, souligné, barré **Faits** ; formes, encre, zones de texte planifiés (P3) |
| Fils de réponses, statut des commentaires | Déclaré | Panneau Commentaires **Fait** (liste, suppression) ; fils et statuts planifiés (P3) |
| Ajout de texte / d'images | Déclaré | **Fait** (texte saisi sur la page, plusieurs lignes, Helvetica/Times/Courier gras/italique ou **police TrueType du système incorporée** (Unicode, caractère absent refusé), taille, couleur, opacité ; image PNG/JPEG/BMP/GIF/TIFF/WebP avec opacité ; formes rectangle/ellipse/ligne avec couleurs, épaisseur et opacité ; droits à l'écran sur les pages pivotées) ; sous-ensemble de polices et `.otf`/`.ttc` planifiés |
| Modification de texte existant | Partiel (faible en CJK/bidi) | **Fait** pour les objets texte simples (édition sur la page ou F2 : texte, taille, couleur ; police d'origine conservée, caractères absents non garantis) |
| Transformation d'objets | Déclaré | **Fait** : déplacement, redimensionnement, **rotation libre** (poignée, angle saisi), duplication, ordre Z (extrême ou d'un cran), recoloration, **opacité**, suppression, **remplacement d'image** ; tests au rendu sur pages normales, pivotées et rognées |
| Sélection multiple, couper/copier/coller d'objets | Déclaré | **Fait** : rectangle, Maj/Ctrl+clic, Ctrl+A, déplacement groupé ; presse-papiers entre pages et onglets (collé = groupe non éditable comme texte) ; rotation/redimensionnement de groupe planifiés |
| Panneau des objets de la page | Déclaré | **Fait** (F3 : ordre de dessin, sélection, avancer/reculer, suppression, repère au survol) |
| Liens | Déclaré | Planifié (P3) |
| Filigranes, en-têtes/pieds de page, numérotation | Partiel | **Fait** : tampon/filigrane texte (angle, opacité, position) et numérotation `{n}`/`{total}` (UI + CLI `stamp`, `page-numbers`) ; filigrane image et en-têtes multi-zones planifiés (P3) |
| Tampons personnalisés | Non documenté | Planifié (P3) |

## C. Organisation

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| Rotation, suppression, insertion, déplacement, pages vierges | Déclaré | **Fait** (UI + CLI `insert-blank` pour les pages vierges) |
| Recadrer les pages (CropBox) | Déclaré | **Fait** (dialogue en mm avec aperçu, rétablissement, CLI `crop`) ; masque seulement, pas de caviardage ; recadrage à la souris planifié |
| Redimensionner les pages | Déclaré | **Fait** (A3–Legal ou personnalisé, contenu conservé ou mis à l'échelle ; CLI `resize-pages`) ; annotations non mises à l'échelle |
| Fusion de N PDF avec signets | Déclaré | Fusion **Faite** (UI « Combiner », CLI `merge`) ; signets de fusion planifiés |
| Division (tous les n / pages choisies) | Déclaré | **Fait** (UI dialogue + CLI `split`, `extract`) |
| Extraction préservant liens, champs, calques, pièces jointes | Déclaré | Extraction **Faite** (UI + CLI, `FPDF_ImportPages`) ; préservation à vérifier avec qpdf |
| Réorganisation par glisser-déposer | Déclaré | **Fait** (miniatures et vue Organiser) |
| Undo/redo nommé survivant à la sauvegarde | Déclaré | Undo/redo **Fait** (instantanés, non nommé) ; survit à la sauvegarde |
| Comparaison de deux versions | Déclaré (crate compare) | Planifié (P5) |
| Traitement par lots | Déclaré (CLI batch) | Planifié (P4/P5) |

## D. Conversion et OCR

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| PDF → images | Déclaré (CLI render) | **Fait** (UI export PNG 150 ppp, CLI `render`, `export-images`) |
| PDF → texte | Déclaré | **Fait** (CLI `text`) |
| Images → PDF | Déclaré (create) | **Fait** (UI + CLI `images2pdf`, PNG/JPEG/BMP/GIF/TIFF/WebP) |
| PDF → Word | Partiel (perd la mise en page) | Planifié (P4), fidélité à annoncer honnêtement |
| PDF → Excel / PowerPoint | Absent / export pptx partiel | Planifié (P4) |
| Office → PDF | Absent | Planifié (P4) via LibreOffice/Word installé si présent ; sinon hors périmètre |
| HTML → PDF | Absent | Planifié (P5, optionnel) |
| OCR | Partiel (latin, modèles CC-BY-SA) | Planifié (P4) : Tesseract (Apache-2.0) fra+eng, hors ligne |

## E. Sécurité et signatures

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| Ouvrir chiffré, honorer les permissions | Déclaré | **Fait** (ouverture ; permissions affichées dans les propriétés) |
| Ajouter/modifier mot de passe, AES-256 | Déclaré | **Fait** (AES-128/256, permissions) via `lopdf`, relu par PDFium ; vérification qpdf à ajouter |
| Caviardage irréversible | Déclaré (crate redact) | Planifié (P5) avec test d'absence des données |
| Nettoyage des métadonnées | Déclaré | **Fait** (Info + XMP) |
| Signature manuscrite (image) | Déclaré | Planifié (P3), jamais présentée comme signature certifiée |
| Signature numérique CMS/PKCS#12, horodatage | Déclaré | Planifié (P5) |
| PKCS#11, LTV en ligne, AATL/EUTL | Absent | Hors périmètre initial |

## F. Formulaires et accessibilité

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| Remplir AcroForm, apparences régénérées | Déclaré | Planifié (P3) via `FPDF_FORMFILL` |
| Créer des champs | Non documenté | Planifié (P3) |
| Import/export FDF/XFDF | Déclaré (xfdf) | Planifié (P3) |
| XFA | Partiel | Hors périmètre |
| Balisage, vérifications PDF/UA | Partiel / absent | Planifié (P6, vérifications seulement) |

## G. Automatisation et intégration

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| CLI JSON | Déclaré | **Fait** (lecture, pages, sécurité, conversion, tampons et édition du contenu : `objects`, `add-text`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`, `crop`, `resize-pages`, `insert-blank`, `fonts`) ; extension P5 |
| Registre de commandes commun UI/CLI | Déclaré (tool table) | **Fait** (`sunipdf-cli commands`) |
| Serveur MCP local, confiné à `--root` | Déclaré | Planifié (P5, opt-in) |
| Intégration Explorateur Windows (association, menu contextuel) | Non documenté | Planifié (P5) |
| Palette de commandes | Déclaré | **Fait** (Ctrl+K, filtrage, clavier, couleur de l'outil propriétaire) |
| Hub « Tous les outils » par tâches, barre contextuelle par outil | Déclaré (liste « All tools ») | **Fait** pour 9 outils sur 14 (Exporter, Modifier, Créer, Combiner, Organiser, Commenter, Protéger, Compresser, Tampon) : panneau F9, cartes d'accueil, menu Outils, palette, barre d'outil avec Échap ; Remplir et signer, OCR, Caviarder, Formulaire, Certificat planifiés (absents tant que leur fonction n'existe pas) |
| Workflows locaux, profils de conversion | Absent | Planifié (P5) |

## H. Distribution

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| Windows x64 | Déclaré (MSI, zip) | Planifié (P6) ; CI écrite, non exécutée |
| Windows ARM64 | Déclaré (workflow dédié) | Planifié (P6) ; binaires PDFium ARM64 disponibles |
| Signature de code | Non documenté | Planifié (P6) si certificat fourni |
