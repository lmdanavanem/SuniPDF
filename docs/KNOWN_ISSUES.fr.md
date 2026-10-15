# Problèmes connus

[English](KNOWN_ISSUES.md)

## Environnement de build (ce poste)

1. ~~Pas de Build Tools~~ **Résolu le 2026-10-13** : Build Tools installés, override GNU retiré, tests verts en MSVC.
   Le repli GNU + WinLibs reste documenté dans `scripts/dev-env.ps1` pour un poste sans droits admin.
2. **Mémoire limitée (14 Go, ~1,5 Go libres pendant la session).** Avec 16 cœurs, `cargo` lance 16 `rustc`
   et le système atteint sa limite de commit : abandons `0xC0000409` et erreurs trompeuses
   (« only metadata stub found », ICE). Contournement : `cargo build -j 2` (ou `CARGO_BUILD_JOBS=2`)
   et `debug = "line-tables-only"` en profil dev.
3. (Repli GNU seulement) **Les binutils livrés par rustup ne suffisent pas** pour les crates `raw-dylib`
   (`dlltool` sans `as`) et le rustc GNU abandonne sous clippy sur le crate `windows` : WinLibs complet requis,
   clippy limité aux crates sans dépendance `windows`.

## Dépendances

4. **wgpu/DX12 désactivé** : `wgpu-hal` 30.0.1 ne compile pas avec `gpu-allocator` 0.28 (D010).
   Backend glow/OpenGL utilisé. Impact possible : machines sans pilote OpenGL correct (VM, ARM64).
   Piste : `cargo update -p gpu-allocator --precise 0.27.x` ou eframe plus récent.
5. **pdfium-render 0.9.4 cible le build PDFium 7881** ; les builds plus récents (8100 au 2026-10-12)
   fonctionnent probablement mais ne sont pas testés. Version épinglée dans `scripts/fetch-pdfium.ps1`.

11. ~~MSI non construit~~ **Résolu le 2026-10-14** : binaires WiX 3.14 autonomes (`third_party/wix314`, zip officiel, sans
    installation ni droits admin) ; `scripts/package-msi.ps1` les détecte via `WIX_BIN` ; MSI x64 produit et validé
    (ICE `smoke.exe`, extraction administrative).
12. **Build ARM64 local bloqué par un composant Visual Studio manquant** (tenté le 2026-10-14) : la cible Rust
    `aarch64-pc-windows-msvc` est installée et le PDFium ARM64 se télécharge, mais l'édition de liens échoue car
    les Build Tools n'ont que les outils x64 et x86 (`VC\Tools\MSVC\<version>\bin\Hostx64\arm64` absent, CRT ARM64
    absente). Installation (droits administrateur requis, refusée pendant la session autonome) :
    `& "C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe" modify --installPath
    "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools" --add Microsoft.VisualStudio.Component.VC.Tools.ARM64
    --passive --norestart`, puis `scripts\package-portable.ps1 -Arch arm64` (depuis Git Bash).
    Les runners GitHub `windows-latest` ont ce composant : `release.yml` construit l'ARM64 en CI.

## Application

A1. **Icônes Fluent** : lues dans `C:\Windows\Fonts\SegoeIcons.ttf` (ou `segmdl2.ttf`). Sur un Windows sans ces
    polices (cas rare : LTSC allégé), les icônes sont remplacées par des symboles texte, moins élégants mais lisibles.
A2. ~~Affichage page par page~~ **Résolu (SUNI-012b)** : défilement continu par défaut, page unique en option (F5).
A3. ~~Rendu à fort zoom limité à 16 384 px~~ **Résolu pour l'écran (SUNI-012b)** : rendu en tuiles jusqu'à
    3200 % (côté de page rendue <= 131 072 px). Corrige aussi un plantage latent : une page rendue d'un seul
    tenant pouvait dépasser la taille de texture maximale du GPU (panique d'`egui_glow` si le pilote limite à
    8192 px) ; aucune texture ne dépasse plus `max_texture_side`. La limite de 16 384 px reste vraie pour
    `Document::render_page`, l'export d'images et `sunipdf-cli render` sans `--region`.
A4. **Chiffrement** : AES-128/256 écrit par lopdf et relu par PDFium dans les tests (aller-retour) ; RC4 40 bits
    par fixture. Aucune fixture AES produite par un outil tiers (qpdf) : à ajouter avec la vérification qpdf.
A5. **La boîte de dialogue d'ouverture (`rfd`) est modale et bloque la boucle UI** le temps du choix :
    comportement natif Windows, acceptable.
A6. **Édition du contenu : la modification d'un texte existant réutilise sa police d'origine.** Si cette police
    (souvent un sous-ensemble incorporé) ne contient pas un caractère saisi, PDFium le remplace ou l'omet.
    Contournement : supprimer l'objet et en ajouter un nouveau (Helvetica, accents latins pris en charge).
A7. ~~Rotation par pas de 90°, pas de police système~~ **Résolu (SUNI-031c)** : rotation libre et polices TrueType
    du système. Restent : **rotation et redimensionnement d'un groupe** de plusieurs objets non proposés
    (déplacement seulement) ; un objet pivoté d'un angle non multiple de 90° n'a plus de poignées d'angle
    (le redimensionner le déformerait) ; pivot = centre de la boîte englobante, donc un tracé asymétrique peut
    dériver légèrement à chaque rotation successive.
A8. **Réordonner (arrière-plan, reculer, avancer) fusionne les flux de contenu de la page** (lopdf) avant de
    réordonner : PDFium dessine flux par flux. Le PDF reste valide. Les objets **dupliqués ou collés sont des
    groupes** (XObjects formulaires) : non éditables comme texte, couleur et opacité non modifiables (champ
    Opacité grisé), bornés à la boîte de la page d'origine (un objet qui dépassait de sa page est rogné).
A18. **Polices système incorporées en entier** (pas de sous-ensemble) : chaque police ajoute sa taille au PDF
     (0,2 à 1,6 Mo pour les polices Windows courantes), une seule fois par document ; après Annuler / Rétablir,
     la première insertion suivante l'incorpore à nouveau et l'ancienne copie reste orpheline jusqu'à
     « Optimiser ». Seules les polices `.ttf` à contours TrueType sont proposées (`.otf` PostScript, `.ttc` et
     polices dont la licence interdit l'incorporation : refus typé `font_not_embeddable`). Liste lue par
     `reg.exe` (repli : dossier des polices) ; un nom non ASCII peut s'afficher mal (page de code OEM). Rendu
     vérifié par PDFium seulement (aucun autre lecteur ni qpdf sur ce poste).
A19. **Recadrer masque, n'efface pas** : le contenu hors cadre reste dans le fichier (ce n'est pas un
     caviardage ; dit dans le dialogue). Recadrage à la souris non proposé (marges numériques avec aperçu).
A20. **Texte sur la page** : le champ de saisie affiche le texte avec la police de l'interface (Segoe UI ou
     monospace pour Courier), pas avec la police du PDF : largeur et retours à la ligne approximatifs pendant la
     saisie. Un texte existant pivoté s'édite dans une boîte de dialogue. Pas d'édition de paragraphes (une ligne
     = un objet).
A21. **Redimensionner en mode « Adapter »** : le contenu est mis à l'échelle (`FPDFPage_TransFormWithClip`),
     pas les annotations ni les liens existants (note permanente dans le dialogue).
A22. **Opacité d'une image existante non lisible** : PDFium 7881 ne renvoie pas la couleur (donc l'alpha) d'un
     objet image ; une image transparente garde dans la barre d'objet la valeur précédente du champ Opacité.
     L'application d'une opacité fonctionne (vérifiée au rendu).
A23. **Angle affiché des objets d'une page pivotée** : l'angle d'un objet est celui de son contenu à l'écran ;
     un rectangle dessiné dans le PDF sur une page `/Rotate 90` est donc annoncé à 90° (CLI `objects`, champ
     Angle). Les poignées d'angle restent disponibles (multiple de 90°).
A24. **Presse-papiers d'objets et presse-papiers système** : Ctrl+V ne colle les objets que si le repère écrit
     par la copie (texte des objets, ou « [SuniPDF : n objet(s) copié(s)] ») est encore dans le presse-papiers
     Windows ; le menu Édition, la palette et la barre d'objet collent sans cette condition. Le presse-papiers
     d'objets vit en mémoire le temps de la session (perdu à la fermeture de l'application).
A9. **Tests moteur sérialisés** : PDFium ne peut pas être réinitialisé dans un même processus après
    `FPDF_DestroyLibrary` ; les tests partagent un moteur unique jamais détruit (`tests/engine.rs`).
A10. **CI GitHub vérifiée pour la 0.8.3** : CI x64 et build ARM64 réussis. La seconde compilation x64 de release
     a été annulée ; le MSI provient des binaires CI vérifiés. Six fichiers publics et leurs SHA256 contrôlés.
     L'installation réelle et l'exécution sur matériel ARM64 restent à valider.
A11. **Hub « Tous les outils » : 11 outils par tâche sur 14.** Numériser et OCR, Préparer un
     formulaire (création de champs) et Utiliser un certificat n'ont pas de fonction réelle : ils sont absents
     (entrée `None` dans `workspace::TABLE`), pas grisés. Exporter, Créer, Combiner et Compresser n'ont pas de
     barre (une seule commande chacun, boîte de dialogue directe). Caviarder est livré.
A12. **Interface du hub vérifiée sans rendu au pixel automatisé** : pas de rendu logiciel ni d'`egui_kittest`
     dans les dépendances. Les tests couvrent les données (table, registre, contraste des glyphes), la bascule
     d'outil, la géométrie des lignes, l'arbre AccessKit et l'anneau de focus (passe egui sans fenêtre) ; le
     rendu réel est contrôlé par captures (`scripts/dev-screenshot.ps1`). Les glyphes de repli (sans Segoe
     Fluent Icons) des nouvelles icônes ne sont pas vérifiés visuellement (voir A1).
A13. **Barre d'outil en fenêtre étroite** : la largeur des libellés est estimée avant le dessin ; sous la
     largeur nécessaire, icônes seules, puis mode compact (nom et « Quitter l'outil » sans libellé) et
     défilement horizontal des boutons (barre de défilement flottante, au survol ; la molette verticale ne fait
     pas défiler, Maj+molette oui). **Défilement non démontré** : à 100 % d'échelle et à la largeur minimale
     de la fenêtre (720 px), la barre la plus longue (Modifier) tient en mode compact ; le défilement n'est
     atteint qu'avec une échelle d'affichage Windows plus grande et n'a été vérifié ni par un test ni par une
     capture.
A14. **Clavier** : après une activation au clavier (Entrée) d'une ligne ou d'un bouton du hub ou de l'accès
     rapide (Ouvrir, Enregistrer, Annuler, Rétablir), le focus est relâché pour que les raccourcis à une
     lettre restent actifs ; Tab repart donc du début. Les boutons egui standard (barre d'onglets, menu)
     n'affichent qu'un indice de focus discret ; les lignes, cartes et boutons du hub ont un anneau de 2 px.
     Échap agit dans l'ordre (un Échap, un effet ; `workspace::EscState::action`, testé) : menu ou liste
     déroulante ouverts, champ ou recherche, objet sélectionné, outil actif. Deux Échap reçus dans la même
     image (impossible au clavier, possible par injection) peuvent encore produire deux effets.
A15. ~~Rail droit en fenêtre basse : chevauchement~~ **Corrigé (SUNI-012b)** : sous ~700 px de haut, le groupe du
     haut (panneaux, bascule de vue, recherche) défile (molette au survol) au lieu de chevaucher la navigation ;
     ses dernières icônes sont alors masquées tant qu'on ne fait pas défiler. Seules des icônes entières sont
     montrées et une fine barre de défilement, visible sans survol, signale la suite (capture 1000 x 560).
A16. **Aide de la CLI pour `stamp` et `page-numbers`** : clap remplace toute séquence `{n}` d'un texte d'aide
     par un saut de ligne (`clap_builder` 4.6, sans échappement possible) ; l'aide affiche donc « : ⏎ = page
     number » au lieu de `{n}`. Les variables `{n}` et `{total}` fonctionnent ; seul le texte d'aide est
     tronqué (relevé par la revue du hub, non traité).
A17. ~~Indicateur « modifié » après annulation~~ **Corrigé (review-fixes, D052)** : point d'enregistrement dans
     l'historique ; perdu seulement si l'on modifie après une annulation ou si l'instantané enregistré sort des 50
     niveaux (le document reste alors « modifié » jusqu'au prochain enregistrement).
A29. **Extraire / diviser un document protégé** : les fichiers produits ne sont pas chiffrés (PDFium copie les
     pages dans un document neuf) ; l'interface et la CLI (`encryption_removed`) le disent, et Protéger
     s'applique ensuite au fichier créé (D046). Rechiffrement automatique en feuille de route.
A30b. **AES-256 et mots de passe** : le mot de passe est normalisé SASLprep avant chiffrement (espaces spéciaux
    ramenés à l'espace, ligatures décomposées), puis limité à un préfixe UTF-8 valide de 127 octets. SuniPDF
    applique la même transformation à l'ouverture ; un autre lecteur peut exiger cette forme effective
    (par exemple « pass word » pour une saisie avec espace insécable). Depuis 0.8.0, un mot de passe d'ouverture
    devenu vide est refusé ; les restrictions exigent des mots de passe effectifs distincts.
A30. **AES-128 et mots de passe** : seul le Latin-1 imprimable est accepté (0x20–0x7E, 0xA1–0xFF sauf 0xAD ;
     lettres accentuées comprises). Les symboles propres à PDFDocEncoding hors Latin-1 (€, œ, Œ, ligatures,
     tirets et guillemets typographiques…) sont refusés (`password_not_latin1`) bien que lopdf les code : PDFium
     reçoit le mot de passe en UTF-8 et ne réessaie qu'en Latin-1, SuniPDF ne rouvrirait donc pas le fichier
     (vérifié par `review_aes128_latin1_passwords_round_trip_through_pdfium`). Choisir AES-256 pour ces
     mots de passe. AES-128 utilise les 32 premiers octets encodés, complétés selon le format PDF ; depuis 0.8.0,
     la différence propriétaire/utilisateur est vérifiée sur ces octets effectifs.
A31. **Images → PDF** : les JPEG sont incorporés tels quels sauf pour les orientations EXIF 5 et 7 (rotation +
     miroir), décodées ; PNG, TIFF, BMP, GIF et WebP sont décodés en mémoire et stockés en pixels compressés
     (Flate), donc plus lourds que l'original. Un très grand lot de PNG reste limité par la mémoire du poste.
     Un JPEG CMJN incorporé tel quel est converti en RVB par PDFium au rendu (rouge pur → 238,29,35) :
     conversion normale d'un flux DCT, pas d'inversion.
A32. **Texte ajouté** : une tabulation vaut quatre espaces (aucune police PDF n'a de glyphe de tabulation).
     Le panneau Commentaires nomme 17 sous-types d'annotation ; les autres (son, vidéo, 3D, filigrane,
     caviardage…) s'affichent « Autre annotation ».
A33. **Point d'enregistrement** : « modifié » s'efface en annulant jusqu'à l'état enregistré, sauf si une
     modification a suivi une annulation (l'état enregistré est alors inatteignable) ou si l'instantané
     enregistré est sorti des 50 niveaux d'historique (D052).
A34. **Propriétés et PDF/A** : modifier les métadonnées régénère un paquet XMP minimal (D048) qui ne reprend pas
     l'identification PDF/A (`pdfaid:part`, `pdfaid:conformance`) ni les autres espaces de noms du paquet
     d'origine : un fichier PDF/A perd son identification après `set-metadata`, sans avertissement.
A25. **Vue continue : limites (SUNI-012b)**.
     - Une **sélection de texte reste sur sa page** (le glissement est borné à la page d'origine) : pas de
       sélection à cheval sur deux pages, pas de défilement automatique quand on glisse près du bord.
     - Défilement **sans animation ni inertie** pour les « aller à » et le clavier (saut direct).
     - **Pincement** (pavé tactile ou écran tactile) : même chemin que Ctrl + molette (`zoom_delta`), non testé
       (aucun test sur ce poste).
       Ctrl + molette vérifié par capture (`dev-screenshot.ps1 -Wheel`).
     - Un **rendu de tuile déjà commencé ne s'interrompt pas** : en défilant vite, la latence d'annulation est
       la durée d'une tuile (mesurée : 1,2 ms par tuile 1024² sur `many-pages.pdf`, plus pour une page lourde).
     - Recollage des tuiles : les glyphes qui traversent un bord de tuile peuvent différer d'un pixel
       d'anticrénelage du rendu d'un seul tenant (121 pixels sur 1,9 million mesurés, D044).
     - Double page, mode livre et rotation de la vue : non proposés (feuille de route).
A26. **« Ajuster » en défilement continu sur un document de tailles mixtes** : le zoom est commun à toutes les
     pages et ajuste la **plus grande** page (D041) ; une page portrait parmi des pages paysage est donc plus
     étroite que l'écran. En page unique, l'ajustement suit la page courante.
A27. **Très longs documents** : positions en `f32` ; le zoom est plafonné pour que la hauteur totale reste
     <= 8 000 000 px logiques (message « Zoom maximal atteint pour ce document »). Un document de 5000 pages
     Lettre est ainsi limité à environ 150 %.
A28. **Mémoire de la vue** : bornée mais notable. Textures des pages 192 Mio (hors textures de l'écran), cache du
     moteur 256 Mio (tuiles et miniatures), 256 textures de miniatures au plus dans le panneau et la grille
     Organiser. Les demandes de miniatures en attente sont aussi plafonnées à 256 ; les résultats périmés
     sont écartés. Tests de régression sur une grille de 5 000 pages. La mesure suivante est antérieure
     à la 0.8.3 et concerne la lecture : build de débogage, 500 pages,
     300 « Page suiv. » puis Fin puis 300 « Page préc. » : mémoire privée stable vers 810-830 Mo (WorkingSet
     438 Mo) après 175 Mo à l'ouverture ; pas de croissance au-delà (pilote OpenGL NVIDIA, copies système des
     textures comprises). Plafonds en constantes (`view.rs`, `worker.rs`), pas de réglage utilisateur.
A35. **Signature manuscrite = signature visuelle, sans valeur cryptographique** (tranche sign-image). C'est une image
     posée sur la page : elle ne prouve ni l'identité du signataire ni l'intégrité du document, et quiconque modifie
     le fichier peut la copier ou la retirer. Pas de certificat, de CMS/PAdES, d'horodatage ni de vérification
     (crate `sunipdf-signatures`, P5). La boîte le dit en permanence ; la CLI `add-image` renvoie
     `"digital_signature": false`. Aucun avertissement non plus si le document porte déjà une signature numérique,
     que toute modification invalide (D056).
A36. **Signatures enregistrées en clair** : PNG dans `%APPDATA%\SuniPDF\signatures` (`signature-<ms>.png`,
     `initials-<ms>.png`), protégés par les seules ACL du profil Windows : toute personne ayant accès à la session peut
     les réutiliser. Parade : décocher « Enregistrer sur cet ordinateur » (rien n'est écrit) ou supprimer la vignette
     (corbeille, en deux clics). Seules les 12 plus récentes par type sont listées ; le tri suppose des horodatages de
     même longueur (13 chiffres, jusqu'en 2286).
A37. **Pavé et import** : trait d'épaisseur constante (egui ne transmet pas la pression du stylet), polyligne brute
     sans lissage ; suppression du fond par **seuil de luminance global** à rampe douce (une photo très inégalement
     éclairée demanderait un seuil adaptatif : juger sur l'aperçu, baisser ou monter le seuil, ou préférer un PNG
     transparent) ; images importées limitées à 16 384 px de côté et 512 Mio décodés (refus à la lecture de
     l'en-tête, même borne de 512 Mio pour Insérer une image, Remplacer l'image et Images → PDF), puis réduites
     à 1600 px de côté ; la date posée est maintenue dans la page sur une largeur **estimée** (0,556 em par
     caractère, chiffres Helvetica), non mesurée ; pas de signature **tapée** (police manuscrite : dépend de
     l'incorporation de polices, A7) ; parapher toutes les pages d'un coup n'est pas proposé (Maj+clic page par
     page) ; bibliothèque pleine : le corps de la boîte défile et sa barre flottante reçoit une gouttière de 10 px
     (le contenu se resserre d'autant à l'image qui suit l'apparition de la barre).
A38. **Formulaires AcroForm (tranche forms, D061–D066)** : **XFA** détecté et lu (champs AcroForm de secours),
     jamais rempli ni aplati (`form_xfa`) ; **scripts** (`/CO`, `/AA`, `/A` JavaScript) jamais exécutés, signalés
     par un bandeau, les champs calculés et les formats restent tels quels ; **pas de création de champs**
     (« Préparer un formulaire » absent du hub) ; **pas de FDF/XFDF** (JSON seulement) ; **un niveau d'annulation
     par valeur validée** (un formulaire de 40 champs = 40 niveaux ; plafonds 50 niveaux / 512 Mo du journal) ;
     chaque validation = instantané complet + analyse lopdf + rechargement + rendu minuscule, acceptable sous
     5 Mo, lent au-delà (mode « Appliquer » groupé en feuille de route) ; polices : un texte hors Latin-1 reçoit de
     PDFium une police **système non incorporée** (Arial, SimSun…), rendu variable sur une autre machine ;
     liste déroulante **modifiable** (`/Ff` Edit) : seuls les choix prévus sont proposés dans l'interface (texte
     libre par la CLI `--set` seulement) ; valeurs d'export `/Opt` des boutons radio ignorées (l'état du
     widget est la valeur) ; texte riche `/RV` remplacé par du texte simple ; cases à cocher à états multiples
     simplifiées (un état actif par widget) ; aplatissement partiel : les champs de la page aplatie restent dans
     le formulaire sans emplacement ; aplatissement : les **liens** (`/Link`) sont relevés avant et remis dans
     `/Annots` après (PDFium les supprimerait sans les dessiner), les autres annotations sans apparence
     (fenêtres `/Popup`, pièces jointes sans `/AP`…) disparaissent avec le contenu dessiné de leur parent ; un
     lien qui porte un `/AP` est dessiné dans le contenu **et** conservé comme lien (même rendu) ; champ **mot de
     passe** : listé, jamais rempli ni exporté ; les widgets ne sont cliquables sur la page qu'avec l'outil Remplir (le
     panneau sert au clavier : Tab / Maj+Tab entre champs texte et listes depuis l'éditeur posé sur la page,
     pas d'activation d'un widget de la page par Espace ou Entrée) ; **Narrateur Windows non testé** (noms
     AccessKit posés : `a11y.forms.*`) ; la page reste une image pour un lecteur d'écran, seuls les rectangles
     de champs et le panneau lui sont exposés.
A39. **Annotations avancées (tranche annotations-advanced, D067–D076)** : **texte libre en Helvetica WinAnsi**
    seulement (accents latins, € et ponctuation courante ; tout autre caractère s'affiche « ? » dans l'apparence,
    `/Contents` garde le texte exact ; pas de gras, d'italique, de texte riche `/RC`, de CJK ni de droite à gauche) et
    largeur de renvoi à la ligne calculée avec la table Helvetica (marge de 2 %) ; **un trait d'encre = une annotation**
    et chaque opération (trait, déplacement, style) coûte un aller-retour complet instantané + lopdf + rechargement :
    imperceptible sur un PDF courant, de l'ordre de la seconde sur un fichier de dizaines de Mo ; **fils de réponses et
    statuts** écrits selon ISO 32000-1 mais **jamais testés dans Acrobat, Foxit ni Chrome** (réponses sans icône :
    apparence vide + `F 4`), pas de modèle d'état « Marked » ; **lecture détaillée** des annotations (panneau,
    sélection) demandée à la demande et par page, qui relit tout le document ; un lien existant ne change pas de cible
    (supprimer puis recréer), destinations nommées non résolues, destination `/XYZ` réduite à la page, actions
    Launch / JavaScript / GoToR jamais exécutées ; **redimensionnement et style réservés aux annotations de SuniPDF**
    (`/NM` préfixé) : celles d'un autre producteur se déplacent, se commentent, se suppriment (surlignages et notes se
    recolorent aussi) ; pas de gomme, de polygone, de nuage, de tampon, de pièce jointe, de bordure pointillée ;
    plafonds : 64 traits de 4 000 points, épaisseur ≤ 50 pt, zone de lien ≥ 6 pt ; **vérifié en capture** (`dev-screenshot.ps1`) : encre,
    sélection d'une annotation de SuniPDF (cadre, quatre poignées, barre flottante) puis déplacement et
    redimensionnement à la souris, déplacement d'une annotation d'un autre producteur (cadre sans poignées), boîtes de
    dialogue (zone de texte, lien, confirmation d'adresse), panneau Commentaires avec fil, thèmes clair et sombre ;
    **non rejoués en capture** : la zone de texte posée après sa boîte, la modification par double-clic, les flèches du
    clavier, le redimensionnement d'une encre, d'une ligne ou d'une zone de texte (tous couverts par les tests moteur et
    les tests purs de `annot_adv.rs`) ; `qpdf --check` non
    exécuté sur les sorties (qpdf absent de ce poste, A4).
A40. **Caviardage (tranche redaction, D077–D082)** : le contenu sous une zone est **supprimé** (objets retirés, fichier
    réécrit sans révision antérieure, vérifié par relecture), mais avec ces limites :
    1. **Granularité** : un objet texte touché par une zone quitte la couche texte en entier ; ce qui dépasse de la
       zone reste visible sous forme d'image (vignette à 200 ppp), ni sélectionnable ni trouvable. Idem pour une
       image ou un tracé partiellement couvert.
    2. **Texte d'une image numérisée sans OCR** : invisible pour « Caviarder un texte… » (il faut tracer la zone). Une
       couche OCR décalée par rapport à l'image peut échapper à une zone tracée : l'option « Convertir entièrement
       les pages caviardées en image » la supprime dans tous les cas.
    3. **Polices incorporées** : si la police du texte caviardé sert encore ailleurs, son sous-ensemble de glyphes
       reste dans le fichier (il révèle des caractères utilisés, ni leur ordre ni leur position).
    4. **Non modifiés** : signets (un titre peut reprendre le texte caviardé), pièces jointes, JavaScript,
       destinations nommées, contenu des pages non marquées ; une annotation `/Redact` posée par un autre logiciel est
       traitée comme une annotation ordinaire (retirée seulement si elle est sous une zone).
    5. **Champs de formulaire** : un champ dont un widget est sous une zone disparaît partout (tous ses widgets,
       toutes les pages) ; `/XFA` est alors retiré.
    6. **Balisage d'accessibilité** : supprimé pour tout le document quand « informations cachées » est coché (défaut)
       ; conservé sinon, avec le risque qu'un texte alternatif décrive le contenu caviardé.
    7. **Signatures numériques** : toute signature existante devient invalide (réécriture complète).
    8. **Mémoire et disque** : l'historique d'annulation et les onglets suspendus gardent l'état d'avant en mémoire
       vive (jamais écrit sur disque par SuniPDF) ; l'ancien fichier remplacé par l'enregistrement atomique n'est pas
       effacé du support de façon sécurisée.
    9. **Hypothèse restante** : les limites d'objet de PDFium englobent l'encre de l'objet. Pour le texte, elles sont
       agrandies de 30 % et contrôlées à l'exécution (texte résiduel sous une zone = erreur, document inchangé) ; pour
       images et tracés elles sont géométriques.
    10. **Documents refusés** : un flux de contenu qui ne se décode pas strictement (lopdf) donne
        `redaction_incomplete` sans rien modifier ; réessayer avec « page en image ».
    11. **Supprimer un objet avec l'outil Modifier n'est pas un caviardage** : PDFium peut laisser l'objet orphelin ou
        la ressource dans le fichier ; seul l'outil Caviarder assainit et réécrit le document.
    12. Les zones sont **effacées à toute modification du document** (rotation, page supprimée, annulation…) ; les
        zones dessinées n'ont pas de nœud d'accessibilité propre (leur nombre est annoncé dans la barre d'état).
    13. Non vérifié : `qpdf --check` des sorties (A4), corpus de PDF réels (numérisations, PDF balisés lourds), pages de
        plusieurs dizaines de milliers d'objets (`prune_objects` de lopdf est quadratique).
