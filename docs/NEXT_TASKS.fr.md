# Prochaines tâches (ordre conseillé)

[English](NEXT_TASKS.md)

## Version 0.8.3 : backlog P1/P2 terminé
Les six priorités précédentes de l'audit, les croix des onglets, les mises à jour intégrées et l'anglais par défaut
restent livrés. Le français reste disponible. La 0.8.3 termine les quatre corrections restantes :

| Priorité | Correction | Couverture de régression |
|---|---|---|
| P1 | Refuser les coordonnées non finies lors de l'ajout de texte | NaN/infini refusés avant mutation ; sortie existante préservée |
| P1 | Borner les miniatures de la grille Organiser | Plafonds des textures et demandes sur 5 000 pages ; résultats périmés écartés |
| P2 | Refuser les tampons et modèles de numérotation multilignes | CR/LF refusés avant mutation, même sans page sélectionnée |
| P2 | Refuser l'opacité non prise en charge du texte libre dans la CLI | Seule l'opacité 1 est acceptée ; opacité non finie refusée ; sortie préservée |

Fonctions suivantes envisagées : sauvegarde automatique/récupération, impression, OCR, création de champs,
signature numérique, double page et sélection de texte enrichie. Elles ne sont pas livrées dans la 0.8.3.

## À faire par l'utilisateur (hors de portée de l'agent)
1. Valider l'interface sur de vrais PDF (accueil, outils, rail droit, palette Ctrl+K, thème sombre, FR/EN) et signaler les écarts.
2. Exécuter le ZIP ARM64 sur une machine ARM64 réelle (jamais fait) et vérifier le rendu OpenGL.
3. Vérifier l'interopérabilité des annotations et du caviardage dans Acrobat, Foxit et Chrome.
4. Valider une mise à jour complète MSI et portable, avec annulation et documents modifiés.

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
Outils encore absents : Numériser et OCR, Préparer un formulaire (création de champs), Utiliser un
certificat. « Remplir et signer » est complet : moitié « signer » (tranche sign-image) et moitié « remplir »
(tranche forms, groupe Remplir le formulaire / panneau / Effacer / Aplatir / Exporter / Importer).

## Après redaction (caviardage irréversible, tranche livrée)
Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A40, décisions D077–D082) :
- `qpdf --check` des sorties et essai sur un corpus réel (numérisations avec couche OCR, PDF balisés lourds,
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
- **création de champs** (texte, case à cocher) par lopdf seul : widget sans `/AP` dans `/Annots` et
  `/AcroForm /Fields`, apparence générée par PDFium (constat V11 de `docs/plans/forms.md` §3.11), outils
  `ToolAddTextField` / `ToolAddCheckbox` et boîte « Nouveau champ » → passe `Workspace::PrepareForm` à
  `Entry::Mode` ; tests : nom unique sans `.`, rectangle ≥ 4 pt, pages pivotées (`rect_to_user`), document
  sans AcroForm ;
- export XFDF (sans dépendance) puis import XFDF/FDF (lecteur XML à choisir) ;
- mode « Appliquer » groupé (plusieurs champs, un seul niveau d'annulation et un seul rechargement) ;
- texte libre dans une liste déroulante modifiable ; valeurs d'export `/Opt` des radios ; activation clavier
  (Espace / Entrée) des rectangles de champs de la page ; vérification Narrateur Windows (noms `labelled_by`
  posés, non écoutés) ;
- aplatissement : conserver aussi les annotations sans apparence autres que les liens (pièces jointes sans
  `/AP`, écrans) si un cas réel le demande ; aujourd'hui seuls les `/Link` sont remis en place ;
- avertir quand le document porte une signature numérique (D056, toute modification l'invalide).

## Après sign-image (signature visuelle, tranche livrée)
Reste, non bloquant (voir `KNOWN_ISSUES.fr.md` A35–A37, décisions D054–D060) :
- signature **tapée** (nom rendu dans une police manuscrite) : dépend de l'incorporation de polices (A7) ;
- parapher **toutes les pages** d'un coup (aujourd'hui Maj+clic, page par page) ;
- chiffrement DPAPI des signatures enregistrées (ADR à écrire, D055) ;
- avertir quand le document porte déjà une **signature numérique** (toute modification l'invalide, D056) ;
- lissage des traits (Chaikin) si les courbes rapides paraissent anguleuses ; seuil adaptatif pour les photos à
  ombre portée ; pression du stylet si egui l'expose un jour.

Reste du hub (non bloquant) : focus clavier placé sur la ligne active à l'ouverture du panneau (F9) ; rail droit
qui se chevauche sous ~650 px de haut (`KNOWN_ISSUES.fr.md` A15) ; validation visuelle finale par l'utilisateur.

## Après review-fixes (constats des revues adverses du 14/10, tranche livrée)
Les 39 constats de `docs/plans/review-fixes.md` sont corrigés ou déjà couverts (tests moteur et application).
Reste, non bloquant :
- rechiffrer automatiquement les extraits et les parties d'une division d'un document protégé (D046, A29) ;
- vérifier avec qpdf les fichiers AES produits (fixture tierce, A4) et les XMP régénérés (lecture par un
  indexeur XMP) ;
- reprendre `pdfaid:*` et les autres espaces de noms du XMP d'origine dans le paquet régénéré par
  `set_metadata` (A34) ;
- incorporer les JPEG d'orientation EXIF 5 et 7 sans décodage (A31) ; limiter la mémoire des lots de PNG ;
- aide CLI de `stamp` / `page-numbers` (A16, clap remplace `{n}`).

## SUNI-012c — Affichage, suite (owner : UI-UX)
Fait : onglets (SUNI-012a) ; vue continue virtualisée, tuiles, zoom ancré, page unique en option (SUNI-012b,
`docs/plans/continuous-view.md`, D038–D045).
Reste (voir `KNOWN_ISSUES.fr.md` A25–A28) :
- sélection de texte à cheval sur plusieurs pages et défilement automatique en glissant au bord ;
- double page / mode livre, rotation de la vue ;
- défilement animé des « aller à » ; pincement tactile à vérifier sur un écran tactile ;
- si une page lourde fige le défilement : `Document::render_regions` (une page chargée pour 3 à 4 tuiles
  voisines), à n'ajouter qu'après mesure ; plafonds mémoire réglables si un besoin apparaît.

## SUNI-013b — Sélection et copie de texte (owner : UI-UX)
Fait : sélection rectangulaire, copie (Ctrl+C réparé en SUNI-031c via l'événement Copier d'egui).
Reste : sélection en ordre de lecture (glisser au fil du texte), double-clic sur un mot.

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
  Acrobat, Foxit et Chrome (non disponibles sur le poste de développement) ; `qpdf --check` des sorties (SUNI-025) ;
- regroupement des traits d'encre (une annotation pour N traits, validée à l'inactivité) avec écriture garantie à
  l'enregistrement et à la fermeture ; gomme ;
- polygone, polyligne, nuage, tampon, pièce jointe, bordure pointillée, texte libre riche (gras, italique, CJK : police
  incorporée) ;
- modifier la cible d'un lien existant, résoudre les destinations nommées, respecter `/XYZ` (position et zoom) ;
- aller-retour allégé pour les gros fichiers (écrire l'annotation sans recharger tout le document) ;
- zone de texte posée directement sur la page (éditeur en place, comme l'outil Texte) au lieu d'une boîte de dialogue.

## SUNI-025 — Compression avec perte et vérification qpdf (owner : OCR-CONVERSION + QA)
Rééchantillonnage des images (PDFium : objets image → DynamicImage → réencodage JPEG), vérification qpdf en CI, fixtures AES produites par qpdf, test « sauvegarde interrompue ».

## SUNI-015 — Identité visuelle et ressources Windows (owner : WINDOWS-RELEASE)
Icône originale (SVG → ICO), manifeste DPI-aware, ressources de version (`winres`/`embed-resource`), signature différée.

## SUNI-016 — QA indépendante (owner : QA-PERFORMANCE)
Corpus de PDF libres (pdf.js test corpus, licences vérifiées), tests fichiers hostiles (sans pages, xref cassé,
chemins Unicode, disque plein simulé), premier benchmark (ouverture + rendu page 1) avec matériel noté.

## Dette technique connue
- Réactiver wgpu/DX12 (D010) ; vérifier OpenGL sur ARM64.
- `-j 2` imposé par la mémoire du poste ; documenter la machine de référence.
- Option `--lang` de la CLI (D009) si des messages humains deviennent nécessaires.
