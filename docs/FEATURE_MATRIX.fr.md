# Matrice fonctionnelle : attendu d'un éditeur PDF complet / SuniPDF

[English](FEATURE_MATRIX.md)

Colonne « Attendu » : capacité attendue d'un éditeur PDF complet (**Déclaré** = courante dans les éditeurs du marché,
**Partiel** = annoncé comme incomplet par le projet, **Absent** = annoncé manquant. Non vérifié par exécution.

Statuts SuniPDF : **Fait** = implémenté **et** couvert par des tests exécutés ; **En cours** ; **Planifié (Pn)** = phase
de la roadmap ; **Hors périmètre**. Mettre à jour cette colonne uniquement avec des résultats réels.

Les nouveautés 0.9.0 sont couvertes par des régressions automatisées. `PROJECT_STATUS.fr.md` suit les contrôles exécutés ;
ces résultats ne prouvent pas la compatibilité imprimante physique, ARM64 ou lecteur tiers.
Workspace, qpdf et régressions réelles portable/MSI 0.9.1 réussis ; résultats des paquets et de publication consignés dans ce suivi.

[La 0.10.0 est publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0) avec le lot P1/P2 demandé.
Les 362 tests du workspace, 31 contrôles qpdf synthétiques, quatre documents NIST réels (744 pages), huit scénarios portables,
deux bases MSI et six fichiers publics ont été vérifiés ; preuves dans `PROJECT_STATUS.fr.md`.

[0.11.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0) sous `lmdanavanem` ; Latest à cette date.

La source 0.11.0 réussit 390 tests du workspace (zéro échec, sept ignorés par défaut), formatage/Clippy avec
`-D warnings`, 858 clés EN/FR, 44 contrôles qpdf sur 42 PDF distincts et quatre documents NIST réels
(744 pages ; 91,59 s, pic mémoire enfant 329 347 072 octets). [CI Windows finale](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860176578) et
[build ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860179447/job/116840434621) réussis, ainsi que neuf scénarios portables réels
depuis 0.8.2/0.8.3/0.9.0/0.9.1/0.10.0, trois bases MSI (0.8.0/0.9.1/0.10.0) et les six fichiers préparés.
Contrôles embarqués : copies AES-128/256 ouvertes comme propriétaire, contenu déchiffré/qpdf, libellés, métadonnées,
OCR, formulaires et compression. Source immuable : `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`. Les six
téléchargements publics concordent avec les empreintes API/SHA256/paquets ; contrôles renforcés réussis sur les
binaires publics. Le metteur à jour de production a vérifié le ZIP Latest x64 de 19 687 375 octets sans installation.
Preuves et limites dans `PROJECT_STATUS.fr.md` ; source/paquets 0.10.0 publiés inchangés.

[0.12.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) sous `lmdanavanem` (Latest à cette date), release
`414674886`, source `3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. Validation réussie : 398 tests workspace (zéro échec,
sept ignorés par défaut), formatage/Clippy, 859 clés EN/FR, 48 contrôles qpdf et quatre documents NIST/NBS (744 pages ;
59,68 s). [CI Windows](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688) et
[job ARM64 de même source](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878) réussis,
ainsi que dix scénarios portables, quatre bases MSI, six fichiers préparés/publics et contrôles renforcés des binaires.
Copies vérifiées dans `dist/`. L'updater de production a vérifié le ZIP Latest x64 de 19 699 323 octets sans installation.
Limites matérielles, d'interface interactive et de lecteurs dans `PROJECT_STATUS.fr.md`.

[0.13.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) sous `lmdanavanem` (Latest à cette date), release
`414763678`, source `b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. Validation réussie : 410 tests workspace (zéro échec,
sept ignorés par défaut), formatage/Clippy, 870 clés EN/FR, 53 contrôles qpdf et quatre documents licenciés
(744 pages ; 76,40 s localement), CI Windows et compilation ARM64 de même source, onze scénarios portables et
cinq bases MSI. Six fichiers préparés/publics et contrôles renforcés des binaires réussis, dont lots de signatures
protégés et les deux mots de passe. Copies vérifiées dans `dist/` ; l'updater de production a vérifié le ZIP Latest
x64 de 19 730 578 octets sans installation. Preuves détaillées et limites restantes dans `PROJECT_STATUS.fr.md`.

[0.14.0 publiée](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) sous `lmdanavanem` (Latest à cette date), release
`414812776`, source `af532888804c196aabe216b8f6a2980c3d1e9e2d`. Validation réussie : 420 tests workspace (zéro échec,
sept ignorés par défaut), formatage/Clippy, 890 clés EN/FR, 63 contrôles qpdf et quatre documents licenciés
(744 pages ; 53,91 s localement), CI Windows et compilation ARM64 de même source, douze scénarios portables et
six bases MSI. Six fichiers préparés/publics et contrôles renforcés des binaires réussis, dont création/réinitialisation
non chiffrée/protégée, les deux mots de passe, étiquettes hors plage et texte/PNG inchangés. Copies vérifiées dans
`dist/` ; l'updater de production a vérifié le ZIP Latest x64 de 19 785 200 octets sans installation.
Preuves et limites restantes dans `PROJECT_STATUS.fr.md`.

[0.15.0 publiée en Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) sous `lmdanavanem`, release
`414866395`, source `952f8699cac451e7c3004b6d816105d52e41f249`. Validation réussie : 429 tests workspace (zéro échec,
sept ignorés par défaut), formatage/Clippy, 894 clés EN/FR, 68 contrôles qpdf et quatre documents licenciés
(744 pages ; 48,891 s localement), CI Windows et compilation ARM64 de même source, treize scénarios portables et
sept bases MSI. Six fichiers préparés/publics et contrôles renforcés réussis, dont cibles non chiffrées/protégées,
les deux mots de passe, identité/zone des liens et PNG des pages inchangés. Copies vérifiées dans `dist/` ; le metteur
à jour de production a vérifié le ZIP Latest x64 de 19 803 226 octets sans installation.
Preuves et limites restantes dans `PROJECT_STATUS.fr.md`.

## A. Lecture

| Fonction | Attendu | SuniPDF | Note |
|---|---|---|---|
| Ouvrir un PDF, afficher une page | Déclaré | **Fait** | PDFium ; tests `crates/sunipdf-core/tests/engine.rs` |
| Documents protégés par mot de passe | Déclaré (toutes révisions) | **Fait** | RC4, AES-128 et AES-256 couverts par les tests moteur/protection et rouverts par PDFium |
| Navigation page/précédente/suivante/aller à | Déclaré | **Fait** (UI) | Clavier + barre d'outils |
| Zoom, ajuster largeur/page, taille réelle | Déclaré | **Fait** (UI) | Rendu à la résolution physique (DPI) ; Ctrl + molette ancré sous le pointeur, 10 à 3200 % |
| Rendu hors thread UI | Déclaré | **Fait** | Thread moteur unique, liste de tuiles voulues remplacée en bloc, époques anti-obsolescence |
| Onglets multi-documents | Déclaré | **Fait** (suspension des documents inactifs, état d'interface conservé) | |
| Affichage continu, double page, rotation de vue | Déclaré | Continu/double page/Livre avec couverture seule **Faits** ; rotation de vue **faite en 0.10.0** | Tuiles/cache communs, régressions tailles mixtes, dernière page et zoom |
| Rendu en tuiles, cache plafonné | Déclaré | **Fait** (tuiles de 1024 px au-delà de 4096 px / 8 Mpx, cache LRU 256 Mo, textures 192 Mo) | `Document::render_region`, tests de pixels (régions, recollage, zoom ×40) ; CLI `render --region` |
| Recherche plein texte | Déclaré | **Fait** | Ctrl+F, casse, surbrillance, CLI `search` |
| Sélection / copie de texte en ordre de lecture | Déclaré | **Fait** : plage logique, sélection entre pages/dans les deux sens, double-clic sur mot, défilement au bord | Ordre PDFium ; tests moteur/UI sans fenêtre ; colonnes arbitraires non balisées non déduites |
| Miniatures | Déclaré | **Fait** | Panneau virtualisé, cache partagé |
| Signets (outline) | Déclaré | **Fait** (panneau, saut de page ; édition planifiée) ; destinations précises **livrées en 0.11.0** | Nommées, XYZ/Fit/FitH non nul ; tests locaux réussis |
| Étiquettes de pages | Déclaré | **Lecture livrée en 0.11.0** ; création/réinitialisation **livrées en 0.14.0**, contrôles locaux/CI/paquets/publics/mises à niveau réussis | Navigation exacte/unique ou `#N` physique conservée ; Pages/Organiser et deux commandes CLI appliquent six styles sur une plage physique inclusive, règles hors plage conservées ; réinitialisation globale ; aucun changement de numéros imprimés ni recalage sémantique |
| Historique de navigation, onglets rouverts | Déclaré | Fichiers récents et historique borné à 256 **Faits** ; historique même page/zoom **livré en 0.11.0** | Tests locaux réussis |
| Mode lecture / plein écran | Déclaré | **Fait en 0.10.0** | Mode de vue partagé comme préférence ; rotation/historique conservés par onglet ; retour à une vue droite pour éditer |
| Impression Windows | Déclaré (fidélité limitée) | **Fait** (tests géométrie/plages et sortie réelle Microsoft Print to PDF rouverte) | Boîte native, plages/copies, rendu tramé borné, progression/annulation ; imprimante physique et boîte interactive non vérifiées |
| Récupération après plantage, autosave | Déclaré | **Fait** (régressions récupération/worker/onglets) | Instantanés 60 s/suspension d'onglet ; source intacte ; chiffrement conservé/sans mots de passe ; restauration Enregistrer sous |
| Thèmes clair/sombre | Déclaré | **Fait** | Design system `theme.rs`, bascule dans la barre d'onglets |
| Interface traduisible | Déclaré (EN, JA + 22 catalogues) | **Fait** (en-US par défaut, fr-FR disponible) | Changement à chaud, préférences existantes conservées |

## B. Édition et annotation

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Notes, surlignage, souligné, barré, formes, encre, zones de texte | Déclaré | **Fait** (D067–D076) : notes, surlignage, souligné, barré ; **encre** à main levée (lissée), **zone de texte libre** (Helvetica WinAnsi), **rectangle, ellipse, ligne, flèche** en vraies annotations PDF (apparence `/AP` explicite, rendues par les autres lecteurs), couleur, épaisseur, fond, opacité ; sélection, déplacement, redimensionnement (annotations de SuniPDF), recoloration, suppression ; pages tournées et rognées justes ; document chiffré conservé chiffré ; CLI `annotate` / `annotations`. **En développement pour 0.16.0** : tirets des quatre formes, motif UI/CLI fixe `[6,3]`, `dash_pattern` personnalisé conservé, style malformé refusé et régénération bornée ; validation en cours (D107) |
| Fils de réponses, statut des commentaires | Déclaré | **Fait dans SuniPDF** : panneau Commentaires en fils (réponses, statut Accepté / Refusé / Annulé / Terminé, auteur, date, suppression du fil), écrits selon ISO 32000-1 ; **compatibilité Acrobat / Foxit non vérifiée** ; CLI `reply` / `review-state` |
| Ajout de texte / d'images | Déclaré | **Fait** (texte saisi sur la page, plusieurs lignes, Helvetica/Times/Courier gras/italique ou **police TrueType du système incorporée** (Unicode, caractère absent refusé), taille, couleur, opacité ; image PNG/JPEG/BMP/GIF/TIFF/WebP avec opacité ; formes rectangle/ellipse/ligne avec couleurs, épaisseur et opacité ; droits à l'écran sur les pages pivotées) ; sous-ensemble de polices et `.otf`/`.ttc` planifiés |
| Modification de texte existant | Partiel (faible en CJK/bidi) | **Fait** pour les objets texte simples (édition sur la page ou F2 : texte, taille, couleur ; police d'origine conservée, caractères absents non garantis) |
| Transformation d'objets | Déclaré | **Fait** : déplacement, redimensionnement, **rotation libre** (poignée, angle saisi), duplication, ordre Z (extrême ou d'un cran), recoloration, **opacité**, suppression, **remplacement d'image** ; tests au rendu sur pages normales, pivotées et rognées |
| Sélection multiple, couper/copier/coller d'objets | Déclaré | **Fait** : rectangle, Maj/Ctrl+clic, Ctrl+A, déplacement groupé ; presse-papiers entre pages et onglets (collé = groupe non éditable comme texte) ; rotation/redimensionnement groupés livrés en 0.10.0, une annulation |
| Panneau des objets de la page | Déclaré | **Fait** (F3 : ordre de dessin, sélection, avancer/reculer, suppression, repère au survol) |
| Liens | Déclaré | **Fait** : création page/URI, adresses autorisées confirmées et navigation nommée/XYZ/Fit/FitH non nul résolue par PDFium en 0.11.0. **Livré en 0.15.0** : édition de cibles existantes UI/CLI, zone/style/commentaires conservés, URI autorisée ou page physique Fit et drapeaux de verrouillage stricts ; contrôles locaux/CI/paquets/publics/mises à niveau réussis (D106). Vue détaillée remplacée explicitement ; Launch/JavaScript/GoTo distant jamais exécutés |
| Filigranes, en-têtes/pieds de page, numérotation | Partiel | **Fait** : tampon/filigrane texte (angle, opacité, position) et numérotation `{n}`/`{total}` (UI + CLI `stamp`, `page-numbers`) ; filigrane image et en-têtes multi-zones planifiés (P3) |
| Tampons personnalisés | Non documenté | Planifié (P3) |

## C. Organisation

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Rotation, suppression, insertion, déplacement, pages vierges | Déclaré | **Fait** (UI + CLI `insert-blank` pour les pages vierges) |
| Recadrer les pages (CropBox) | Déclaré | **Fait** (dialogue en mm avec aperçu, rétablissement, CLI `crop`) ; masque seulement, pas de caviardage ; recadrage souris dans l’aperçu livré en 0.10.0 |
| Redimensionner les pages | Déclaré | **Fait** (A3–Legal ou personnalisé, contenu conservé ou mis à l'échelle ; CLI `resize-pages`) ; annotations non mises à l'échelle |
| Fusion de N PDF avec signets | Déclaré | Fusion **Faite** (UI « Combiner », CLI `merge`) ; signets de fusion planifiés |
| Division (tous les n / pages choisies) | Déclaré | **Fait** (UI dialogue + CLI `split`, `extract`) |
| Extraction préservant liens, champs, calques, pièces jointes | Déclaré | Extraction **Faite** (UI + CLI, `FPDF_ImportPages`) ; **implémenté pour 0.11.0, publié ; contrôles locaux, CI, paquets et mises à niveau réussis** : conserver chiffrement/mots de passe/permissions et refuser une sortie au chemin source (D097 remplace D046). Politique des permissions inchangée ; CLI `encryption_removed: false`, ajout `encrypted` ; préservation sémantique à vérifier au-delà de qpdf |
| Réorganisation par glisser-déposer | Déclaré | **Fait** (miniatures et vue Organiser) |
| Undo/redo nommé survivant à la sauvegarde | Déclaré | Undo/redo **Fait** (instantanés, non nommé) ; survit à la sauvegarde. **Livré en 0.14.0** : navigation conservée lors d'Annuler/Rétablir contenu/étiquettes si géométrie inchangée ; contrôles exécutés réussis (D105) |
| Comparaison de deux versions | Déclaré (crate compare) | Planifié (P5) |
| Traitement par lots | Déclaré (CLI batch) | **Fait en 0.10.0** : compression/OCR séquentiels, 128 PDF, 256 Mio/entrée, progression/annulation/erreurs et nouvelles copies sans écrasement |

## D. Conversion et OCR

| Fonction | Attendu | SuniPDF |
|---|---|---|
| PDF → images | Déclaré (CLI render) | **Fait** (UI export PNG 150 ppp, CLI `render`, `export-images`) |
| PDF → texte | Déclaré | **Fait** (CLI `text`) |
| Images → PDF | Déclaré (create) | **Fait** (UI + CLI `images2pdf`, PNG/JPEG/BMP/GIF/TIFF/WebP ; orientation EXIF appliquée, JPEG incorporés tels quels) |
| PDF → Word | Partiel (perd la mise en page) | Planifié (P4), fidélité à annoncer honnêtement |
| PDF → Excel / PowerPoint | Absent / export pptx partiel | Planifié (P4) |
| Office → PDF | Absent | Planifié (P4) via LibreOffice/Word installé si présent ; sinon hors périmètre |
| HTML → PDF | Absent | Planifié (P5, optionnel) |
| Compression des images | Déclaré | **Faite en 0.10.0** : RGB/gris bornés, qualité JPEG/côté maximal, copie exacte sans gain, masques/profils conservés, PDF signés/restreints refusés |
| OCR | Partiel (latin, modèles CC-BY-SA) | **Fait** : Tesseract 5.5.3 / Leptonica 1.87 embarqués hors ligne, modèles rapides eng/fra, texte invisible, pages textuelles ignorées, progression/annulation/un undo ; tests moteur et reconnaissance CLI réelle ; CLI `ocr` |

## E. Sécurité et signatures

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Ouvrir chiffré, honorer les permissions | Déclaré | **Fait** (ouverture ; permissions affichées dans les propriétés) |
| Ajouter/modifier mot de passe, AES-256 | Déclaré | **Fait** (AES-128/256, permissions ; AES-128 limité aux mots de passe Latin-1, refus typé ; accessibilité toujours accordée) via `lopdf`, relu par PDFium ; longueur AES-256 explicite et contrôle qpdf synthétique ajoutés pour 0.9.1 ; résultats dans `PROJECT_STATUS.fr.md` |
| Caviardage irréversible | Déclaré (crate redact) | **Fait** (outil Caviarder du hub : zones à la souris, marquage d'un texte, confirmation, garde d'enregistrement ; CLI `redact`) : objets retirés + vignette noircie, annotations / champs / métadonnées / révisions antérieures supprimés, vérification par relecture ; absence prouvée par tests (extraction, octets décompressés, pixels, pages pivotées, document chiffré) ; limites `KNOWN_ISSUES.fr.md` A40 |
| Nettoyage des métadonnées | Déclaré | **Fait** (Info, XMP du catalogue, des pages et des images, `/PieceInfo`) ; l'édition 0.9.1 préserve l'identification PDF/A et le XMP indépendant, refuse les XML malformés, validation dans `PROJECT_STATUS.fr.md` ; conformité PDF/A non certifiée |
| Signature visuelle (image) | Déclaré | **Fait** : dessin, saisie, import ou PNG local en clair réutilisé, paraphe/date, image alpha modifiable et undo. **Livré en 0.13.0** : nom/paraphe saisi avec aperçu natif explicite et contrôles de police existants ; page courante/toutes/sélection Organiser, géométrie ajustée, annulation atomique/un undo ; ajout CLI `add-image --pages`. Contrôles locaux, CI, paquets/publics et mises à niveau réussis. **Visuelle, sans preuve d'identité/intégrité** ; parcours CMS/PFX distinct inchangé |
| Signature numérique CMS/PKCS#12, horodatage | Déclaré | Intégrité/copie signée PFX/P12 **Faites** (tests avec certificat jetable) ; CLI `signatures`/`sign` ; PDF chiffré refusé. Confiance/révocation/horodatage fiable/LTV non implémentés ; interopérabilité Acrobat/Foxit non vérifiée |
| PKCS#11, LTV en ligne, AATL/EUTL | Absent | Hors périmètre initial |

## F. Formulaires et accessibilité

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Remplir AcroForm, apparences régénérées | Déclaré | **Fait** (tranche forms, D061–D063) : texte (une ligne, multiligne, longueur maximale), case à cocher, boutons radio, liste déroulante, zone de liste (choix multiple), champs hiérarchiques et à plusieurs widgets ; valeurs écrites par lopdf, apparences régénérées par PDFium et présentes dans le fichier ; outil sur la page + panneau accessible + aplatissement (champs et annotations) + effacer ; document chiffré conservé chiffré ; CLI `form-fields` / `fill-form` / `flatten` ; scripts jamais exécutés (signalés) |
| Créer des champs | Non documenté | **Fait** : boîte numérique texte/case ; radio/listes livrés en 0.10.0 ; noms uniques, pages pivotées/recadrées/chiffrées, annulation ; tests moteur/CLI ; CLI `add-form-field` |
| Import/export FDF/XFDF | Déclaré | **Fait en 0.10.0** : export JSON/XFDF, import strict borné JSON/XFDF/FDF de valeurs, Appliquer/Abandonner groupés et une annulation ; sous-ensemble FDF, aucun script/pièce jointe/texte enrichi |
| Récupération après erreur de lecture des champs | Non documenté | **Livrée en 0.11.0** : erreur/relance explicites préservent brouillons et actions en attente ; tests locaux réussis |
| Texte libre des listes déroulantes déjà modifiables | Non documenté | **Livré en 0.11.0** : texte libre/choix dans le panneau et sur la page, Appliquer/Abandonner/Enregistrer et annulation ; contrôles locaux, CI, paquets et mises à niveau réussis |
| Créer des listes déroulantes modifiables | Non documenté | **Livré en 0.12.0** : case de l'interface et CLI `add-form-field --kind combo --editable` ; choix prédéfinis non vides requis ; contrôles locaux, CI, paquets et mises à niveau réussis |
| Exports radio `/Opt` | Non documenté | **Livrés en 0.12.0** : valeurs d'export distinctes des états des widgets pour remplissage/JSON/XFDF/FDF ; ajout CLI `widgets[].export_value` ; correspondances malformées/dupliquées : champ en lecture seule ; radios sans `/Opt` inchangés ; contrôles locaux, CI, paquets et mises à niveau réussis |
| XFA | Partiel | **Détecté, lecture seule** (champs AcroForm listés ; remplissage et aplatissement refusés, `form_xfa`) |
| Balisage, vérifications PDF/UA | Partiel / absent | Planifié (P6, vérifications seulement) |

## G. Automatisation et intégration

| Fonction | Attendu | SuniPDF |
|---|---|---|
| CLI JSON | Déclaré | **Fait** (lecture, pages, sécurité, conversion, tampons et édition du contenu : `objects`, `add-text`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`, `crop`, `resize-pages`, `insert-blank`, `fonts`) ; extension P5 |
| Registre de commandes commun UI/CLI | Déclaré (tool table) | **Fait** (`sunipdf-cli commands`) |
| Serveur MCP local, confiné à `--root` | Déclaré | Planifié (P5, opt-in) |
| Intégration Explorateur Windows (association, menu contextuel) | Non documenté | Planifié (P5) |
| Palette de commandes | Déclaré | **Fait** (Ctrl+K, filtrage, clavier, couleur de l'outil propriétaire) |
| Hub « Tous les outils » par tâches, barre contextuelle par outil | Déclaré (liste « All tools ») | **Fait** pour les 14 outils ; Numériser et OCR, Préparer un formulaire et Utiliser un certificat ont leurs commandes ; tests registre et activation sans fenêtre |
| Workflows locaux, profils de conversion | Absent | Planifié (P5) |

## H. Distribution

| Fonction | Attendu | SuniPDF |
|---|---|---|
| Windows x64 | Déclaré (MSI, zip) | **Fait** : 0.15.0 ZIP/MSI publiés en Latest ; treize scénarios portables et sept bases MSI réussis ; six fichiers publics/SHA256 et contrôles embarqués vérifiés ; mise à jour de production vérifiée sans installation ; preuves dans `PROJECT_STATUS.fr.md` ; UX interactive non vérifiée |
| Windows ARM64 | Déclaré | **Fait** : ZIP portable 0.15.0 de même source compilé en CI et publié ; SHA256 public, architecture, modèles, notices et icônes vérifiés ; preuves dans `PROJECT_STATUS.fr.md` ; exécution matérielle non vérifiée |
| Signature de code | Non documenté | Planifié (P6) si certificat fourni |
