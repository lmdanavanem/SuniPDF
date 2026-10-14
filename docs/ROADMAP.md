# Roadmap SuniPDF

Chaque phase a un critère de sortie vérifiable par des commandes réelles. Une phase n'est « terminée »
que si ses critères fondamentaux sont atteints et consignés dans `PROJECT_STATUS.md`.

## Phase 0 — Audit et planification — **terminée (2026-10-13)**
Livrables : `PDFCRAFT_AUDIT.md`, `FEATURE_MATRIX.md`, `ARCHITECTURE.md`, `TECH_STACK.md`, `ROADMAP.md`,
`AGENT_STRATEGY.md`, `I18N.md`, `DECISIONS.md`, 12 agents.

## Phase 1 — Fondations — **terminée (2026-10-14)**
- [x] Workspace Rust compilable, application Windows démarrable (cible MSVC ; repli GNU documenté).
- [x] Moteur PDFium intégré, chargé dynamiquement, erreurs typées.
- [x] Ouvrir → afficher → naviguer → zoomer → fermer ; mot de passe ; glisser-déposer ; argument ligne de commande.
- [x] i18n fr-FR/en-US, changement à chaud, préférence persistée (écriture atomique), thème clair/sombre.
- [x] Tests : moteur (7, dont chiffré et fichiers invalides), i18n (6), app (3).
- [x] Compilation MSVC (Build Tools installés le 13/10) ; exécution vérifiée par captures d'écran automatisées.
- [x] Icône et identité visuelle originales (`assets/icon`, `scripts/make_icon.py`).
Critère de sortie : `cargo test --workspace` vert + un PDF réel ouvert et affiché dans l'application.

## Phase 2 — Fonctions essentielles — **largement livrée (2026-10-14)**
Fait : registre de commandes, onglets, cache de rendu + miniatures, recherche, sélection/copie de texte, signets,
fusion/division/extraction/rotation/réorganisation, enregistrement atomique, undo/redo, annotations principales.
Reste : affichage continu, vérification qpdf en CI, tuiles pour le zoom profond (plans : `docs/plans/continuous-view.md`, `compress-lossy.md`).

### Détail initial
Registre de commandes ; onglets ; affichage continu ; cache de rendu plafonné + miniatures ; recherche ;
sélection/copie de texte ; signets ; fusion/division/extraction/rotation/réorganisation ; sauvegarde atomique
vérifiée par qpdf ; undo/redo ; annotations principales (surlignage, note, formes).
Critère : opérations de bout en bout testées (fichier produit vérifié par qpdf + relecture PDFium).

## Phase 3 — Édition professionnelle — **partiellement livrée**
Fait : notes, surlignage/soulignement/barré, commentaires (panneau), métadonnées, propriétés, palette de commandes.
Fait (SUNI-031 à 033) : ajout de texte/images, formes, modification/déplacement/redimensionnement/rotation 90°/duplication/ordre Z/suppression d'objets existants, couleurs, polices standard, tampons, filigranes, numéros de page, dialogues unifiés.
Reste : rotation libre, polices système, formulaires, liens, annotations avancées (encre, texte libre), autosave/récupération, signature image (plans dans `docs/plans/`).

### Détail initial
Ajout de texte/images ; modification de texte selon capacités PDFium ; formulaires AcroForm + FDF/XFDF ;
filigranes, en-têtes/pieds ; liens ; commentaires avec fils ; métadonnées ; autosave/récupération ;
palette de commandes ; signature manuscrite (image, explicitement non certifiée).

## Phase 4 — Conversion et OCR — **partiellement livrée**
Fait : images ↔ PDF, PDF → texte, export PNG, optimisation sans perte, lots via CLI.
Reste : OCR, PDF → Word/Excel, Office → PDF.

### Détail initial
Images↔PDF ; PDF→texte ; OCR Tesseract fra/eng ; PDF→Word/Excel avec niveau de fidélité mesuré et
affiché ; Office→PDF via convertisseur installé (LibreOffice/Word) détecté ; lots.

## Phase 5 — Sécurité et productivité — **partiellement livrée**
Fait : chiffrement AES-128/256 + permissions, retrait de protection, nettoyage des métadonnées, CLI complète
(20 commandes JSON), commandes de démarrage pour l'automatisation.
Reste : caviardage, signatures, comparaison, MCP, intégration Explorateur, profils.

### Détail initial
Chiffrement AES-256 et permissions à l'écriture (vérifiés qpdf) ; caviardage irréversible avec test
d'absence ; nettoyage des métadonnées ; signatures CMS/PKCS#12 + vérification ; compression ; comparaison ;
CLI complète ; MCP opt-in confiné ; intégration Explorateur ; profils de conversion ; raccourcis configurables.

## Phase 6 — Qualité et distribution — **partiellement livrée**
Fait : ZIP portable x64 + SHA256, MSI x64 construit localement (WiX 3.14 autonome) + workflow `release.yml` (x64, ARM64,
signature optionnelle) déclenché par les tags `v*`, ressources Windows, guides utilisateur FR/EN, notices tierces.
Reste : résultat de la CI à confirmer, ARM64 compilé, corpus et fuzzing, benchmarks publiés.

### Détail initial
Corpus de non-régression + fuzzing ; benchmarks publiés avec matériel ; audit sécurité et licences ;
x64 + ARM64 ; MSI (WiX) + ZIP portable + checksums ; CI complète ; documentation utilisateur FR/EN ;
réévaluation d'un repli pur Rust (hayro).

## Fonctions différenciantes (transversales, après P2)
Lot ergonomique, profils enregistrables, recherche multi-documents, OCR multilingue, comparaison côte à côte,
workflows locaux, IA générative strictement optionnelle et désactivée par défaut.
