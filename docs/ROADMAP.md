# Roadmap SuniPDF

Chaque phase a un critère de sortie vérifiable par des commandes réelles. Une phase n'est « terminée »
que si ses critères fondamentaux sont atteints et consignés dans `PROJECT_STATUS.md`.

## Phase 0 — Audit et planification — **terminée (2026-10-13)**
Livrables : `PDFCRAFT_AUDIT.md`, `FEATURE_MATRIX.md`, `ARCHITECTURE.md`, `TECH_STACK.md`, `ROADMAP.md`,
`AGENT_STRATEGY.md`, `I18N.md`, `DECISIONS.md`, 12 agents.

## Phase 1 — Fondations — **en cours**
- [x] Workspace Rust compilable, application Windows démarrable (cible GNU sur ce poste).
- [x] Moteur PDFium intégré, chargé dynamiquement, erreurs typées.
- [x] Ouvrir → afficher → naviguer → zoomer → fermer ; mot de passe ; glisser-déposer ; argument ligne de commande.
- [x] i18n fr-FR/en-US, changement à chaud, préférence persistée (écriture atomique), thème clair/sombre.
- [x] Tests : moteur (7, dont chiffré et fichiers invalides), i18n (6), app (3).
- [ ] Compilation MSVC et exécution manuelle validée par l'utilisateur sur son poste.
- [ ] Icône et identité visuelle originales (fichiers `assets/`).
Critère de sortie : `cargo test --workspace` vert + un PDF réel ouvert et affiché dans l'application.

## Phase 2 — Fonctions essentielles (cible : 3–4 semaines-agent)
Registre de commandes ; onglets ; affichage continu ; cache de rendu plafonné + miniatures ; recherche ;
sélection/copie de texte ; signets ; fusion/division/extraction/rotation/réorganisation ; sauvegarde atomique
vérifiée par qpdf ; undo/redo ; annotations principales (surlignage, note, formes).
Critère : opérations de bout en bout testées (fichier produit vérifié par qpdf + relecture PDFium).

## Phase 3 — Édition professionnelle
Ajout de texte/images ; modification de texte selon capacités PDFium ; formulaires AcroForm + FDF/XFDF ;
filigranes, en-têtes/pieds ; liens ; commentaires avec fils ; métadonnées ; autosave/récupération ;
palette de commandes ; signature manuscrite (image, explicitement non certifiée).

## Phase 4 — Conversion et OCR
Images↔PDF ; PDF→texte ; OCR Tesseract fra/eng ; PDF→Word/Excel avec niveau de fidélité mesuré et
affiché ; Office→PDF via convertisseur installé (LibreOffice/Word) détecté ; lots.

## Phase 5 — Sécurité et productivité
Chiffrement AES-256 et permissions à l'écriture (vérifiés qpdf) ; caviardage irréversible avec test
d'absence ; nettoyage des métadonnées ; signatures CMS/PKCS#12 + vérification ; compression ; comparaison ;
CLI complète ; MCP opt-in confiné ; intégration Explorateur ; profils de conversion ; raccourcis configurables.

## Phase 6 — Qualité et distribution
Corpus de non-régression + fuzzing ; benchmarks publiés avec matériel ; audit sécurité et licences ;
x64 + ARM64 ; MSI (WiX) + ZIP portable + checksums ; CI complète ; documentation utilisateur FR/EN ;
réévaluation d'un repli pur Rust (hayro).

## Fonctions différenciantes (transversales, après P2)
Lot ergonomique, profils enregistrables, recherche multi-documents, OCR multilingue, comparaison côte à côte,
workflows locaux, IA générative strictement optionnelle et désactivée par défaut.
