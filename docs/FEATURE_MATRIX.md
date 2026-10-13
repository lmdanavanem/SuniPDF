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
| Zoom, ajuster largeur/page, taille réelle | Déclaré | **Fait** (UI) | Rendu à la résolution physique (DPI) |
| Rendu hors thread UI | Déclaré | **Fait** | Thread moteur unique, générations anti-obsolescence |
| Onglets multi-documents | Déclaré | **Fait** (suspension des documents inactifs, état d'interface conservé) | |
| Affichage continu, double page, rotation de vue | Déclaré | Planifié (P2) | |
| Rendu en tuiles, cache plafonné | Déclaré | Cache **Fait** (LRU 256 Mo, pré-rendu) ; tuiles planifiées (P2) | |
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
| Ajout de texte / d'images | Déclaré | Planifié (P3) |
| Modification de texte existant | Partiel (faible en CJK/bidi) | Planifié (P3, selon PDFium `FPDFText*` edit) |
| Liens | Déclaré | Planifié (P3) |
| Filigranes, en-têtes/pieds de page, numérotation | Partiel | Planifié (P3) |
| Tampons personnalisés | Non documenté | Planifié (P3) |

## C. Organisation

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| Rotation, suppression, insertion, déplacement, pages vierges | Déclaré | **Fait** (UI + CLI) sauf pages vierges |
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
| CLI JSON | Déclaré | **Fait** (`info`, `render`) ; extension P5 |
| Registre de commandes commun UI/CLI | Déclaré (tool table) | **Fait** (`sunipdf-cli commands`) |
| Serveur MCP local, confiné à `--root` | Déclaré | Planifié (P5, opt-in) |
| Intégration Explorateur Windows (association, menu contextuel) | Non documenté | Planifié (P5) |
| Palette de commandes | Déclaré | **Fait** (Ctrl+K, filtrage, clavier) |
| Workflows locaux, profils de conversion | Absent | Planifié (P5) |

## H. Distribution

| Fonction | PDFCraft | SuniPDF |
|---|---|---|
| Windows x64 | Déclaré (MSI, zip) | Planifié (P6) ; CI écrite, non exécutée |
| Windows ARM64 | Déclaré (workflow dédié) | Planifié (P6) ; binaires PDFium ARM64 disponibles |
| Signature de code | Non documenté | Planifié (P6) si certificat fourni |
