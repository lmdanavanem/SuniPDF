# Prochaines tâches (ordre conseillé)

[English](NEXT_TASKS.md)

## Version 0.9.0 : les sept fonctions demandées sont publiées

Les sept fonctions sont dans la [release 0.9.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.0)
sous `lmdanavanem`. [PROJECT_STATUS.fr.md](PROJECT_STATUS.fr.md) consigne les contrôles exécutés, preuves de publication
et limites matérielles/d'interopérabilité. Les anciens nombres de tests ne sont pas le critère de publication actuel.

| Fonction | Périmètre inclus |
|---|---|
| Sauvegarde et récupération | Instantanés 60 s/suspension, source intacte, PDF protégés toujours chiffrés, aucun mot de passe, restauration Enregistrer sous |
| Impression Windows | Plages/copies/orientation natives, modifications courantes, rendu tramé borné, progression/annulation |
| Sélection enrichie | Plages logiques PDFium entre pages, glissement inverse, double-clic sur mot et défilement au bord |
| Double page/Livre | Tailles mixtes, couverture seule, dernière page isolée, zoom partagé et caches bornés |
| Création de formulaires | Rectangles numériques texte/case, noms uniques, documents pivotés/recadrés/chiffrés, annulation/remplissage/enregistrement |
| OCR local FR/EN | Tesseract 5.5.3/Leptonica 1.87 embarqués, modèles rapides eng/fra, texte invisible, pages textuelles ignorées, annulation atomique et un undo |
| Signatures numériques | Intégrité des octets enregistrés, copie PFX/P12 incrémentale, avertissement de réécriture ; confiance de chaîne/révocation/horodatage fiable/LTV non implémentés |

Les corrections P1/P2 de la 0.8.3 restent incluses : coordonnées finies, miniatures Organiser bornées,
tampons sur une ligne et opacité CLI valide. Anglais par défaut ; français disponible.

## Version 0.9.1 : fiabilité en attente de validation/publication

- Conserver les identifiants PDF/A et le XMP indépendant en modifiant les propriétés standard ; refuser clairement les XML malformés/DTD.
  Cela ne certifie pas la conformité PDF/A.
- Écrire explicitement la longueur AES-256 à l'enregistrement, y compris après métadonnées/nettoyage/optimisation.
- Contrôles qpdf figés en CI sur fixtures synthétiques/sorties ; le corpus réel ou hostile reste un travail distinct.
- Régressions de mise à jour/restauration portable et d'installation/mise à niveau MSI réelle sur runners Windows jetables.
- Icône Windows dédiée dérivée du visuel fourni par l'auteur et retouché.

L'implémentation est présente ; lire les résultats actuels des tests, CI, paquets et publication dans `PROJECT_STATUS.fr.md`.
Ces contrôles ne prouvent pas l'exécution ARM64, les imprimantes physiques, les lecteurs tiers ni l'UX interactive de mise à jour.

## Backlog de développement restant

1. **P1 — QA indépendante :** corpus réel volumineux/hostile, enregistrements interrompus/disque plein, mises à jour
   MSI/portable interactives, matériel ARM64, imprimantes physiques, Narrateur et Acrobat/Foxit/Chrome.
2. Sous-ensembles de polices, CFF/OTF/TTC et édition enrichie des paragraphes/groupes.
3. Ordre de lecture complexe de colonnes non balisées, rotation de vue et validation tactile.
4. Autres types de champs, FDF/XFDF et activation clavier des widgets sur page.
5. Conversion Office avec limites de fidélité explicites et convertisseurs locaux détectés.
6. Confiance/révocation des certificats et horodatage fiable/LTV (non implémentés) ; signature du code/MSI si identité
   fournie. Versions et dates futures demandent un périmètre défini séparément.

## À faire par l'utilisateur (hors de portée de l'agent)
1. Valider l'interface sur de vrais PDF (accueil, outils, rail droit, palette Ctrl+K, thème sombre, FR/EN) et signaler les écarts.
2. Exécuter le ZIP ARM64 sur une machine ARM64 réelle (jamais fait) et vérifier le rendu OpenGL.
3. Vérifier l'interopérabilité des annotations et du caviardage dans Acrobat, Foxit et Chrome.
4. Valider une mise à jour complète MSI et portable, avec annulation et documents modifiés.
5. Vérifier le Narrateur Windows et la navigation clavier dans l'interface réelle.

## Hub d'outils : contrat pour les tranches suivantes (tool-hub livré, D021)
Les onglets `ToolTab` et les cartes `home.card.*` n'existent plus : les plans de `docs/plans/` qui les citent
(OCR, caviardage, export, compression, formulaires, signatures…) passent par `apps/sunipdf-desktop/src/workspace.rs`.
Pour rendre un outil visible, une tranche fait exactement :
1. ajouter ses `Command` (moteur + `REGISTRY`, id stable, `needs_document` correct) ;
2. dans `workspace::TABLE`, passer `entry` à `Some(Entry::Direct(cmd))` ou `Some(Entry::Mode(Command::HubXxx))`,
   renseigner `groups` (chaque commande dans **un seul** outil), choisir l'icône ;
3. si l'outil a une barre : ajouter `HubXxx` au registre (menu `Tools`), au bras `Hub*` d'`execute` et à
   `shell::command_icon` ;
4. les noms, descriptions et couleurs des 14 outils existent déjà (`hub.*.name`, `hub.*.desc`).
Le test `every_registry_command_has_a_home` échoue si une commande n'a ni outil ni place dans la liste système ;
`bar_icons_are_distinct` échoue si une icône se répète dans une barre ou reprend celle d'un bouton d'accès rapide.
Les 14 outils possèdent une implémentation en 0.9.0, dont Numériser et OCR, Préparer un formulaire et Utiliser un certificat. « Remplir et signer » est complet : moitié « signer » (tranche sign-image) et moitié « remplir »
(tranche forms, groupe Remplir le formulaire / panneau / Effacer / Aplatir / Exporter / Importer).

## Après redaction (caviardage irréversible, tranche livrée)
Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A40, décisions D077–D082) :
- contrôles indépendants sur un corpus réel (numérisations avec couche OCR, PDF balisés lourds,
  formulaires XObject imbriqués, polices Type 3) ; mesure sur une page de plusieurs dizaines de milliers d'objets
  (`prune_objects` quadratique : accessibilité maison par `BTreeSet` si lent) ;
- découpe fine du texte (garder sélectionnable la partie non caviardée d'une ligne) : demande de recréer des
  fragments avec la police d'origine, hors de portée de `pdfium-render` 0.9.4 ;
- couleur de remplissage et texte en surimpression (« CAVIARDÉ »), marques `/Redact` persistantes (pas de créateur
  dans `pdfium-render`) ;
- nettoyage des signets, pièces jointes, JavaScript et destinations nommées (outil « Assainir » séparé) ;
- suppression d'un sous-ensemble de police devenu inutile ; aperçu « avant / après » de la page ;
- `docs/SECURITY_MODEL.md` (méthode A/B/C, garanties, hypothèses) ; capture S3 (après application) non rejouée.

## Après forms (formulaires AcroForm, tranche livrée)
Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A38, décisions D061–D066) :
- export XFDF puis import XFDF/FDF (réutiliser le lecteur XML existant lorsque pertinent) ;
- mode « Appliquer » groupé (plusieurs champs, un seul niveau d'annulation et un seul rechargement) ;
- texte libre dans une liste déroulante modifiable ; valeurs d'export `/Opt` des radios ; activation clavier
  (Espace / Entrée) des rectangles de champs de la page ; vérification Narrateur Windows (noms `labelled_by`
  posés, non écoutés) ;
- aplatissement : conserver aussi les annotations sans apparence autres que les liens (pièces jointes sans
  `/AP`, écrans) si un cas réel le demande ; aujourd'hui seuls les `/Link` sont remis en place ;

## Après sign-image (signature visuelle, tranche livrée)
Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A35–A37, décisions D054–D060) :
- signature **tapée** (nom rendu dans une police manuscrite) : dépend de l'incorporation de polices (A7) ;
- parapher **toutes les pages** d'un coup (aujourd'hui Maj+clic, page par page) ;
- chiffrement DPAPI des signatures enregistrées (ADR à écrire, D055) ;
- lissage des traits (Chaikin) si les courbes rapides paraissent anguleuses ; seuil adaptatif pour les photos à
  ombre portée ; pression du stylet si egui l'expose un jour.

Reste du hub (non bloquant) : focus clavier placé sur la ligne active à l'ouverture du panneau (F9) ;
validation visuelle finale par l'utilisateur. Le chevauchement du rail droit en fenêtre basse est déjà corrigé (A15).

## Après review-fixes (constats des revues adverses du 14/10, tranche livrée)
Les 39 constats de `docs/plans/review-fixes.md` sont corrigés ou déjà couverts (tests moteur et application).
Reste, non bloquant :
- rechiffrer automatiquement les extraits et les parties d'une division d'un document protégé (D046, A29) ;
- élargir le corpus d'interopérabilité AES/XMP au-delà du contrôle qpdf synthétique (A4) ;
- contrôler les métadonnées PDF/A préservées avec un validateur dédié (A34) ; la correction de préservation 0.9.1
  est implémentée et les preuves d'exécution actuelles sont dans `PROJECT_STATUS.fr.md` ;
- incorporer les JPEG d'orientation EXIF 5 et 7 sans décodage (A31) ; limiter la mémoire des lots de PNG ;
- aide CLI de `stamp` / `page-numbers` (A16, clap remplace `{n}`).

## SUNI-012c — Affichage, suite (owner : UI-UX)
Fait : onglets (SUNI-012a) ; vue continue virtualisée, tuiles, zoom ancré, page unique en option (SUNI-012b,
`docs/plans/continuous-view.md`, D038–D045).
Reste (voir `KNOWN_ISSUES.fr.md` A25–A28) :
- rotation de la vue ;
- défilement animé des « aller à » ; pincement tactile à vérifier sur un écran tactile ;
- si une page lourde fige le défilement : `Document::render_regions` (une page chargée pour 3 à 4 tuiles
  voisines), à n'ajouter qu'après mesure ; plafonds mémoire réglables si un besoin apparaît.

## SUNI-013b — Sélection et copie de texte (owner : UI-UX)
Fait : sélection rectangulaire, copie (Ctrl+C réparé en SUNI-031c via l'événement Copier d'egui).
Implémenté en 0.9.0 : plages logiques PDFium, mots, sélection entre pages et défilement au bord. Reste : déduire un ordre complexe de colonnes non balisées.

## SUNI-031d — Édition du contenu, suite (owner : PDF-EDITOR)
Fait en SUNI-031/031b/032/031c : sélection simple et multiple (rectangle, Maj/Ctrl+clic, Ctrl+A), déplacement
groupé, redimensionnement, rotation libre, duplication, ordre Z (un cran ou extrême), suppression, couleurs,
épaisseur, opacité, copier/couper/coller entre pages et onglets (groupe), panneau Objets, texte sur la page
(polices standard et TrueType du système), remplacement d'image, pages vierges, recadrage, redimensionnement
des pages, pages pivotées et rognées.
Reste (voir `KNOWN_ISSUES.fr.md` A7, A8, A18–A22) :
- sous-ensemble des polices incorporées (crate `subsetter` MIT/Apache + post-traitement lopdf, ADR à écrire) ;
  polices `.otf` (contours CFF) et collections `.ttc` ;
- collage « natif » éditable (objets recréés un par un au lieu d'un XObject) ; opacité et couleur des groupes ;
- rotation et redimensionnement d'un groupe de plusieurs objets ; pivot mémorisé pour les tracés asymétriques ;
- recadrage à la souris ; glisser-déposer dans le panneau Objets ;
- lecture de l'opacité d'une image existante (contournement PDFium : lire l'ExtGState du flux avec lopdf).
Acceptation : chaque opération couverte par un test moteur avec rendu vérifié.

## SUNI-021b — Annotations avancées : livré, reste (owner : PDF-EDITOR)
Livré (D067–D076) : encre, zone de texte libre, rectangle, ellipse, ligne, flèche, liens cliquables, sélection /
déplacement / redimensionnement / recoloration / suppression, fils de réponses et statuts, auteur mémorisé, CLI
`annotations` / `annotate` / `reply` / `review-state` / `annotation-delete`. Reste :
- **vérifier l'interopérabilité** des réponses et statuts, des apparences d'encre, de texte libre et de formes dans
  Acrobat, Foxit et Chrome (non disponibles sur le poste de développement) ; un contrôle structurel ne prouve pas l’interopérabilité visuelle ;
- regroupement des traits d'encre (une annotation pour N traits, validée à l'inactivité) avec écriture garantie à
  l'enregistrement et à la fermeture ; gomme ;
- polygone, polyligne, nuage, tampon, pièce jointe, bordure pointillée, texte libre riche (gras, italique, CJK : police
  incorporée) ;
- modifier la cible d'un lien existant, résoudre les destinations nommées, respecter `/XYZ` (position et zoom) ;
- aller-retour allégé pour les gros fichiers (écrire l'annotation sans recharger tout le document) ;
- zone de texte posée directement sur la page (éditeur en place, comme l'outil Texte) au lieu d'une boîte de dialogue.

## SUNI-025 — Compression avec perte et vérification qpdf (owner : OCR-CONVERSION + QA)
Restent le rééchantillonnage (PDFium : objets image → DynamicImage → JPEG), le corpus réel/hostile et les scénarios
d'enregistrement interrompu/disque plein. Le contrôle qpdf synthétique figé en CI et la longueur AES-256 explicite
sont implémentés pour 0.9.1 ; résultats d'exécution dans `PROJECT_STATUS.fr.md`.

## SUNI-015 — Identité visuelle et ressources Windows (owner : WINDOWS-RELEASE)
Livrés : manifeste DPI-aware et ressources de version. L'icône Windows 0.9.1 dérive du visuel dédié fourni par l'auteur
et retouché `assets/icon/sunipdf-source.png` (ICO + tailles PNG).
Reste : signature du code des exécutables/MSI lorsqu'un certificat sera disponible.

## SUNI-016 — QA indépendante (owner : QA-PERFORMANCE)
Corpus de PDF libres (pdf.js test corpus, licences vérifiées), tests fichiers hostiles (sans pages, xref cassé,
chemins Unicode, disque plein simulé), premier benchmark (ouverture + rendu page 1) avec matériel noté.

## Dette technique connue
- Réactiver wgpu/DX12 (D010) ; vérifier OpenGL sur ARM64.
- `-j 2` imposé par la mémoire du poste ; documenter la machine de référence.
- Option `--lang` de la CLI (D009) si des messages humains deviennent nécessaires.
