# Problèmes connus

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
A10. **Résultat de la CI GitHub non vérifié par l'agent** : dépôt privé, ni `gh` ni jeton sur ce poste. Les
     workflows `ci.yml` et `release.yml` sont déclenchés par les push et les tags ; leur état se lit dans l'onglet
     Actions.
A11. **Hub « Tous les outils » : 9 outils sur les 14 de PDFCraft.** Remplir et signer, Numériser et OCR,
     Caviarder, Préparer un formulaire et Utiliser un certificat n'ont pas de fonction réelle : ils sont absents
     (entrée `None` dans `workspace::TABLE`), pas grisés. Exporter, Créer, Combiner et Compresser n'ont pas de
     barre (une seule commande chacun, boîte de dialogue directe).
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
A30. **AES-128 et mots de passe** : seul le Latin-1 imprimable est accepté (0x20–0x7E, 0xA1–0xFF sauf 0xAD ;
     lettres accentuées comprises). Les symboles propres à PDFDocEncoding hors Latin-1 (€, œ, Œ, ligatures,
     tirets et guillemets typographiques…) sont refusés (`password_not_latin1`) bien que lopdf les code : PDFium
     reçoit le mot de passe en UTF-8 et ne réessaie qu'en Latin-1, SuniPDF ne rouvrirait donc pas le fichier
     (vérifié par `review_aes128_latin1_passwords_round_trip_through_pdfium`). Choisir AES-256 pour ces
     mots de passe.
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
     moteur 256 Mio (tuiles et miniatures), 256 miniatures au plus. Mesuré en build de débogage, 500 pages,
     300 « Page suiv. » puis Fin puis 300 « Page préc. » : mémoire privée stable vers 810-830 Mo (WorkingSet
     438 Mo) après 175 Mo à l'ouverture ; pas de croissance au-delà (pilote OpenGL NVIDIA, copies système des
     textures comprises). Plafonds en constantes (`view.rs`, `worker.rs`), pas de réglage utilisateur.
