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
A4. **Chiffrement et QA indépendante** : AES-128/256 est écrit par lopdf et relu par PDFium ; RC4 40 bits est couvert
    par fixture. En 0.9.1, le helper partagé écrit explicitement `/Encrypt /Length 256` pour AES-256, valeur omise
    auparavant et signalée par qpdf, y compris après rechiffrement. Des contrôles qpdf synthétiques et fixtures AES
    générées par qpdf sont ajoutés en CI. Résultats actuels dans `PROJECT_STATUS.fr.md` ; ce n'est pas un corpus réel/hostile.
A5. **La boîte de dialogue d'ouverture (`rfd`) est modale et bloque la boucle UI** le temps du choix :
    comportement natif Windows, acceptable.
A6. **Édition du contenu : la modification d'un texte existant réutilise sa police d'origine.** Si cette police
    (souvent un sous-ensemble incorporé) ne contient pas un caractère saisi, PDFium le remplace ou l'omet.
    Contournement : supprimer l'objet et en ajouter un nouveau (Helvetica, accents latins pris en charge).
A7. ~~Rotation par pas de 90°, pas de police système~~ **Résolu (SUNI-031c)** : rotation libre et polices TrueType
    du système. Rotation/redimensionnement groupés implémentés en 0.10.0 avec une annulation ; validation dans `PROJECT_STATUS.fr.md`. Restent : un objet pivoté d'un angle non multiple de 90° n'a plus de poignées d'angle
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
     vérifié par PDFium ; compatibilité visuelle des autres lecteurs non vérifiée. qpdf ne valide pas le rendu.
A19. **Recadrer masque, n'efface pas** : le contenu hors cadre reste dans le fichier (ce n'est pas un
     caviardage ; dit dans le dialogue). Recadrage souris dans l’aperçu implémenté en 0.10.0 ; marges numériques conservées.
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
A10. **Validation de distribution** : l'état courant des tests, CI, paquets et releases est consigné dans
     `PROJECT_STATUS.fr.md`. La 0.10.0 publiée réussit huit scénarios portables réels de mise à jour/échec/restauration et
     deux bases MSI sur runners jetables : installation/rollback transactionnel/mise à niveau/refus de downgrade/désinstallation,
     avec conservation des données. Parcours interactifs, matériel ARM64 et imprimantes physiques restent distincts.
A11. **Hub « Tous les outils » : 14 outils implémentés en 0.9.0.** OCR, création de champs et certificat
     ont désormais de vraies commandes. Couverture automatisée et matérielle dans `PROJECT_STATUS.fr.md`.
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
A29. ~~Extraction/division retirant la protection PDF~~ **Résolu en 0.11.0** :
     les copies conservent chiffrement, mots de passe et permissions d'origine via le writer protégé (D097 remplace
     D046). Sortie au chemin source résolu refusée. CLI `encryption_removed: false`, ajout `encrypted`.
     Politique des permissions inchangée ; validations courantes dans `PROJECT_STATUS.fr.md`.
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
A34. **Propriétés et PDF/A** : la 0.9.1 conserve l'identification PDF/A (`pdfaid:part`, `pdfaid:conformance`) et les
     propriétés/espaces de noms XMP indépendants lors de la modification des quatre propriétés standard. Les XML
     malformés, DTD et flux illisibles sont refusés clairement ; aucune sortie ne remplace silencieusement le paquet.
     Conservation de l'identification ne signifie pas certification de conformité PDF/A. Validateur dédié et corpus
     réels restent à ajouter ; preuves d'exécution actuelles dans `PROJECT_STATUS.fr.md`.
A25. **Vue continue : limites (SUNI-012b)**.
     - La sélection suit l’ordre logique PDFium, y compris entre pages et au bord de la vue ; l’ordre voulu
       de colonnes arbitraires non balisées n’est pas déduit. Le marquage reste rectangulaire par page.
     - Défilement **sans animation ni inertie** pour les « aller à » et le clavier (saut direct).
     - **Pincement** (pavé tactile ou écran tactile) : même chemin que Ctrl + molette (`zoom_delta`), non testé
       (aucun test sur ce poste).
       Ctrl + molette vérifié par capture (`dev-screenshot.ps1 -Wheel`).
     - Un **rendu de tuile déjà commencé ne s'interrompt pas** : en défilant vite, la latence d'annulation est
       la durée d'une tuile (mesurée : 1,2 ms par tuile 1024² sur `many-pages.pdf`, plus pour une page lourde).
     - Recollage des tuiles : les glyphes qui traversent un bord de tuile peuvent différer d'un pixel
       d'anticrénelage du rendu d'un seul tenant (121 pixels sur 1,9 million mesurés, D044).
     - Double page et Livre gardent les mêmes plafonds ; rotation de vue/plein écran/historique implémentés en 0.10.0. L’édition revient à une vue droite ; contrôles interactifs distincts.
A26. **« Ajuster » en défilement continu sur un document de tailles mixtes** : le zoom est commun à toutes les
     pages et ajuste la **plus grande** page (D041) ; une page portrait parmi des pages paysage est donc plus
     étroite que l'écran. En page unique, l'ajustement suit la page courante. En double page/Livre, la plus large
     paire et la page la plus haute déterminent le zoom partagé.
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
A35. **Signature visuelle = image sans preuve cryptographique** : dessinée, saisie ou importée, copiable ou supprimable par un éditeur.
     La CLI `add-image` indique `digital_signature: false`. La signature CMS/PFX est un autre parcours dans
     Utiliser un certificat (A44) ; l’application avertit avant la réécriture d’un PDF signé.
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
     caractère, chiffres Helvetica), non mesurée. **0.13.0** : nom/paraphe saisi et aperçu natif
     explicite (une ligne, 256 caractères, 1600 px/côté ; Times italique par défaut, contrôles existants des glyphes
     et droits d'incorporation TrueType standard/système), pose courante/toutes/sélection. Saisie, changement de mode
     ou annulation de boîte/document invalident l'aperçu. Même centre en points PDF affichés, ajusté aux bords visibles,
     réduction seulement si trop grand ; aucun positionnement relatif proportionnel. L'annulation est vérifiée entre
     les pages, pas pendant un appel natif PDFium ; un lot volumineux demande aussi un instantané et une copie temporaire
     en mémoire. Un undo rétablit le lot entier ; la source change seulement à Enregistrer. CLI : ajustement opt-in
     par `add-image --pages`, sémantique historique de `--page` conservée. Publié en 0.13.0 ; contrôles exécutés et
     limites restantes dans `PROJECT_STATUS.fr.md`. Bibliothèque pleine : le corps de la boîte défile et sa barre flottante reçoit une gouttière de 10 px
     (le contenu se resserre d'autant à l'image qui suit l'apparition de la barre).
A38. **Formulaires AcroForm** : XFA lu, jamais rempli/aplati ; JavaScript, calculs et actions jamais exécutés.
     La 0.10.0 implémente création texte/case/radio/listes, export XFDF et import strict borné JSON/XFDF/FDF de valeurs.
     Chaque champ/valeur est validé avant mutation ; une entrée inconnue/invalide refuse tout l’import de l’interface.
     Les modifications du panneau restent en attente jusqu’à Appliquer ou Enregistrer, une seule annulation/relecture ;
     Abandonner revient aux valeurs courantes. La confirmation sur la page reste immédiate. Espace/Entrée activent
     les widgets focalisés ; les listes reçoivent le focus clavier. Noms uniques sans point/contrôle et rectangles
     numériques dans la page, dimensions minimales 4 points. Radios verticaux ; libellés dans le panneau, pas en texte PDF.
     FDF limité aux tables xref ordinaires et valeurs texte/nom/tableau ; flux, actions, pièces jointes, texte enrichi
     et export FDF non pris en charge. XML DTD/instructions de traitement/éléments inconnus/doublons/taille/profondeur
     excessives refusés ; chemins de document référencés jamais ouverts. Import limité à 16 Mio.
     La 0.11.0 livre le texte libre des combos déjà modifiables dans le panneau/sur la page ; contrôles locaux, CI et paquets réussis.
     La 0.12.0 livre la création de listes modifiables (choix prédéfinis requis) et les exports radio `/Opt`
     distincts des états d'apparence. Correspondances malformées/dupliquées : champ concerné en lecture seule ;
     même règle si `/Kids` brut malformé, références de widget dupliquées ou noms d'apparence non UTF-8. Les valeurs
     en lecture seule se réimportent inchangées, y compris `Off` littéral ou chaîne vide. Remplir un autre champ
     préserve `/V` et `/AS` bruts ; PDFium peut régénérer une apparence manquante et retirer la lecture seule déduite
     une fois le champ valide. Sans `/Opt`, noms d'état conservés. Contrôles locaux, CI, paquets/publics et mises à niveau réussis. Modification du drapeau d'un champ existant
     et dessin souris indisponibles. Texte enrichi remplacé par texte simple ; mots de passe jamais stockés/exportés.
     Aplatissement partiel : champs orphelins possibles ; annotations sans
     apparence autres que Link parfois perdues. Apparences Unicode susceptibles d’utiliser une police système non
     embarquée, donc rendu variable ailleurs. Narrateur et lecteurs tiers non vérifiés ; preuves dans `PROJECT_STATUS.fr.md`.


A39. **Annotations avancées (tranche annotations-advanced, D067–D076)** : **texte libre en Helvetica WinAnsi**
    seulement (accents latins, € et ponctuation courante ; tout autre caractère s'affiche « ? » dans l'apparence,
    `/Contents` garde le texte exact ; pas de gras, d'italique, de texte riche `/RC`, de CJK ni de droite à gauche) et
    largeur de renvoi à la ligne calculée avec la table Helvetica (marge de 2 %) ; **un trait d'encre = une annotation**
    et chaque opération (trait, déplacement, style) coûte un aller-retour complet instantané + lopdf + rechargement :
    imperceptible sur un PDF courant, de l'ordre de la seconde sur un fichier de dizaines de Mo ; **fils de réponses et
    statuts** écrits selon ISO 32000-1 mais **jamais testés dans Acrobat, Foxit ni Chrome** (réponses sans icône :
    apparence vide + `F 4`), pas de modèle d'état « Marked » ; **lecture détaillée** des annotations (panneau,
    sélection) demandée à la demande et par page, qui relit tout le document. Navigation nommée/XYZ livrée en
    0.11.0 (A49). **0.15.0** ajoute le remplacement de cible existante UI/CLI (D106) : URI autorisée
    ou page physique Fit, vue détaillée explicitement remplacée. Zone/style/noms/commentaires, réponses et données
    indépendantes conservés ; `A`/`Dest`/`AA` remplacés ensemble. ReadOnly (64), Locked (128), LockedContents (512),
    drapeaux négatifs/malformés ou indirects illisibles : édition refusée. Cible inconnue : adresse vide, jamais page 1.
    Un Annuler conserve géométrie/navigation ; éditions et changements d'onglet ferment les boîtes périmées.
    Publié en 0.15.0 ; contrôles locaux, CI, mises à niveau, paquets préparés/publics et metteur à jour réussis ;
    preuves et limites restantes dans `PROJECT_STATUS.fr.md`.
    Launch / JavaScript / GoToR jamais
    exécutées ; **redimensionnement et style réservés aux annotations de SuniPDF**
    (`/NM` préfixé) : celles d'un autre producteur se déplacent, se commentent, se suppriment (surlignages et notes se
    recolorent aussi). **0.16.0 en développement** ajoute les tirets uniquement aux annotations rectangle, ellipse,
    ligne et flèche. UI/CLI choisissent `[6,3]` points ; vecteur réel conservé à la sélection, recoloration,
    déplacement/redimensionnement et annulation sauf changement explicite de case. Phase zéro, remplissage et
    pointes de flèche pleins. Motifs non vides refusés sur encre/texte libre/lien/marquages. `BS` malformé/non pris
    en charge sur une annotation SuniPDF : édition régénérant l'apparence refusée, sans remplacer le style. Au plus
    16 valeurs finies non négatives, chacune ≤ 1000 pt, valeur positive ≥ 0,1 pt et somme positive ; borne conservatrice
    de 100 000 segments. `D []` explicite : continu valide ; `D` absent : `[3]`. Bordure SuniPDF malformée :
    Déplacer/Redimensionner/Style désactivés. Commentaire de forme autorisé selon les permissions ordinaires sans
    reconstruire l'apparence ; commentaire de texte libre malformé refusé. Aperçu : motifs sous-pixel valides
    conservés, contour non vide invalide/excessif omis sans repli continu, sous la même borne de travail.
    Aucun éditeur de motif/phase. Validation en cours dans `PROJECT_STATUS.fr.md` (D107) ;
    la 0.15.0 reste Latest. Pas de gomme, polygone, nuage, tampon ou pièce jointe ;
    plafonds : 64 traits de 4 000 points, épaisseur ≤ 50 pt, zone de lien ≥ 6 pt ; **vérifié en capture** (`dev-screenshot.ps1`) : encre,
    sélection d'une annotation de SuniPDF (cadre, quatre poignées, barre flottante) puis déplacement et
    redimensionnement à la souris, déplacement d'une annotation d'un autre producteur (cadre sans poignées), boîtes de
    dialogue (zone de texte, lien, confirmation d'adresse), panneau Commentaires avec fil, thèmes clair et sombre ;
    **non rejoués en capture** : la zone de texte posée après sa boîte, la modification par double-clic, les flèches du
    clavier, le redimensionnement d'une encre, d'une ligne ou d'une zone de texte (tous couverts par les tests moteur et
    les tests purs de `annot_adv.rs`) ; contrôle structurel qpdf synthétique ajouté pour 0.9.1 (A4) ;
    interopérabilité des lecteurs toujours non établie.
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
    8. **Mémoire et disque** : annulation et onglets suspendus gardent les états antérieurs en mémoire. Le caviardage
       réussi actualise immédiatement l'instantané courant (ou le retire si l'actualisation échoue), mais les copies
       d'anciennes sessions interrompues peuvent contenir du texte non caviardé. L'original, les sauvegardes et les
       copies supprimées ne sont pas effacés du support de façon sécurisée.
    9. **Hypothèse restante** : les limites d'objet de PDFium englobent l'encre de l'objet. Pour le texte, elles sont
       agrandies de 30 % et contrôlées à l'exécution (texte résiduel sous une zone = erreur, document inchangé) ; pour
       images et tracés elles sont géométriques.
    10. **Documents refusés** : un flux de contenu qui ne se décode pas strictement (lopdf) donne
        `redaction_incomplete` sans rien modifier ; réessayer avec « page en image ».
    11. **Supprimer un objet avec l'outil Modifier n'est pas un caviardage** : PDFium peut laisser l'objet orphelin ou
        la ressource dans le fichier ; seul l'outil Caviarder assainit et réécrit le document.
    12. Les zones sont **effacées à toute modification du document** (rotation, page supprimée, annulation…) ; les
        zones dessinées n'ont pas de nœud d'accessibilité propre (leur nombre est annoncé dans la barre d'état).
    13. Non vérifiés : numérisations/PDF balisés lourds réels et pages de plusieurs dizaines de milliers d'objets
        (`prune_objects` de lopdf est quadratique). Le contrôle qpdf synthétique ne remplace pas ce corpus (A4).


## Limites des fonctions 0.9.0

Les résultats de validation exécutés sont suivis dans `PROJECT_STATUS.fr.md`.

A41. **Récupération** : instantanés toutes les 60 secondes et à la suspension d’un onglet, donc une modification
     plus récente peut manquer après un arrêt forcé. Une saisie de dialogue non validée n’est pas un changement PDF.
     Les instantanés protégés restent chiffrés, mais noms de fichiers/chemins et nombre de pages sont dans les métadonnées JSON.
     Aucun mot de passe n’est conservé ; il faut le ressaisir pour restaurer. L’original n’est jamais remplacé par la récupération.
A42. **Impression Windows** : rendu tramé jusqu’à 300 dpi par bandes bornées, ajusté à la zone imprimable ; pas de
     sortie vectorielle ni de garantie de couleur professionnelle. Le pilote gère copies/orientation. Une opération native
     déjà lancée peut retarder l’annulation. Sortie réelle Microsoft Print to PDF rouverte avec modifications non
     enregistrées, source/octet/objets inchangés. Imprimantes physiques et boîte interactive non vérifiées.
A43. **OCR local** : anglais/français uniquement, modèles rapides Tesseract ; qualité variable selon scan et langue.
     Les pages ayant déjà du texte sont ignorées, même si cette couche est mauvaise. Images limitées à 16 Mpx et 8192 px/côté,
     sortie TSV 16 Mio, 50 000 mots/page, délai 120 s/page ; très grandes pages refusées. Pas de reconstruction sémantique
     des tableaux/colonnes. Annulation/échec ne valide aucune page ; succès en un undo. Autres lecteurs à vérifier.
A44. **Signatures numériques** : CMS détaché `adbe.pkcs7.detached`/`ETSI.CAdES.detached` sur octets enregistrés ;
     signature PFX/P12 incrémentale dans une nouvelle copie, pas sur PDF chiffré. Validité cryptographique ne signifie
     pas confiance de l’identité : chaîne, révocation, AATL/EUTL, PKCS#11, horodatage fiable et LTV non implémentés.
     L’heure du PDF est déclarative. Les modifications/réécritures peuvent invalider une signature. Interopérabilité
     Acrobat/Foxit et qualification PAdES non établies ; ne pas présenter le résultat comme signature qualifiée.

A45. **Compression des images 0.10.0** : avec perte, images RGB/gris 8 bits prises en charge et limitées à 32 millions
     de pixels. Graphes des masques, profils, Decode/DecodeParms particuliers et autres images conservés. Si le fichier
     grossit, restitution exacte des octets source. Aucun poids cible garanti. PDF signés ou restreints refusés ; nouvelle copie.
A46. **Lots 0.10.0** : compression ou OCR anglais/français séquentiels, 128 PDF et 256 Mio/entrée maximum. Aucun fichier
     existant remplacé ; copies terminées conservées après erreur/annulation. Pas de demande de mot de passe par fichier ;
     document qui en nécessite un refusé. Annulation aux points de contrôle ; codec/appel natif actif peut retarder l’arrêt.
A47. **Sauvegardes/corpus 0.10.0** : erreurs de stockage injectées et arrêt forcé d’une écriture partielle contrôlent
     la préservation de la source. Ce ne sont pas toutes les pannes physiques/coupures possibles. Quatre PDF NIST publics
     complètent les fixtures synthétiques : 31 contrôles qpdf synthétiques et quatre documents réels (744 pages) réussis en 0.10.0.
     Cela ne prouve pas une couverture client/hostile/volumineuse universelle ; preuves exécutées dans `PROJECT_STATUS.fr.md`.

## Changements publiés et limites 0.11.0

Ces ajouts sont publiés en [0.11.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0). Contrôles locaux, CI, paquets et mises
à niveau réussis ; contrôles publics/mise à jour aussi réussis ; preuves et limites dans `PROJECT_STATUS.fr.md`.

A48. **Étiquettes de page** : métadonnées de navigation facultatives, distinctes des numéros imprimés. Un libellé
     vide/illisible, dépassant 4096 octets UTF-8 ou contenant un caractère de contrôle revient à l'indice physique.
     Le libellé exact et unique prime sur une saisie numérique ; `#N` choisit la page physique N. Un doublon exige
     un indice physique. Aucun recalage sémantique après une opération structurelle : les plages peuvent rester
     attachées aux positions physiques ; l'extraction peut omettre les libellés. Les indices CLI restent physiques, à partir de 1.
     **0.14.0** ajoute création par plage et réinitialisation globale via Pages/Organiser et CLI.
     Six styles, nouveaux préfixes sur une ligne de 256 octets UTF-8 au plus, étiquettes générées de 4096 octets au
     plus et départ positif. Parcours des arbres numériques borné ; arbre malformé/dangereux : Appliquer refusé
     avant mutation. Règles hors plage et continuation à leur frontière conservées. Réinitialiser retire l'entrée
     du catalogue, même malformée ; aucun effacement sécurisé des anciens objets. L'avertissement existant de
     réécriture des PDF signés s'applique. Annuler/Rétablir conserve la navigation si la géométrie reste inchangée
     (D104–D105) ; éditions structurelles positionnelles sans recalage sémantique. Publié en 0.14.0 ; contrôles
     exécutés et limites restantes dans `PROJECT_STATUS.fr.md`.
A49. **Navigation vers les destinations** : liens internes/signets avec `/XYZ`, `/Fit` et positions `/FitH` exploitables,
     historique de position/zoom par onglet. Les autres ajustements conservent la navigation à la page seule.
     PDFium ne distingue pas une position `/FitH` absente de zéro ; les deux reviennent à la page seule. Des détails
     illisibles/invalides font de même. Axes `/XYZ` absents et zoom nul/omis conservent les valeurs courantes ; un zoom
     explicite est borné par l'application. Cela n'établit pas l'interopérabilité Acrobat/Foxit ni celle des lecteurs d'écran natifs.
A50. **Échec de lecture du formulaire** : distinct d'un formulaire vide, avec une nouvelle tentative explicite depuis
     le panneau. Brouillons et valeurs en attente sont conservés pendant l'échec. Un import/effacement en attente
     reprend seulement après une lecture réussie ; les nouvelles tentatives automatiques répétées sont bloquées.
