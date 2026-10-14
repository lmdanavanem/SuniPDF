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
A2. **Affichage page par page** : la vue continue (défilement de toutes les pages) est planifiée
    (`docs/plans/continuous-view.md`). Onglets, cache de rendu LRU et miniatures existent.
A3. **Rendu à fort zoom** limité à 16 384 px de côté (`MAX_RENDER_SIDE_PX`) ; au-delà, message d'erreur.
    Le rendu en tuiles lèvera cette limite.
A4. **Chiffrement** : AES-128/256 écrit par lopdf et relu par PDFium dans les tests (aller-retour) ; RC4 40 bits
    par fixture. Aucune fixture AES produite par un outil tiers (qpdf) : à ajouter avec la vérification qpdf.
A5. **La boîte de dialogue d'ouverture (`rfd`) est modale et bloque la boucle UI** le temps du choix :
    comportement natif Windows, acceptable.
A6. **Édition du contenu : la modification d'un texte existant réutilise sa police d'origine.** Si cette police
    (souvent un sous-ensemble incorporé) ne contient pas un caractère saisi, PDFium le remplace ou l'omet.
    Contournement : supprimer l'objet et en ajouter un nouveau (Helvetica, accents latins pris en charge).
A7. **Édition du contenu : rotation par pas de 90° seulement, pas de police système** (14 polices standard) ;
    les images sont insérées à 200 pt de large puis redimensionnables par les poignées
    (`docs/plans/edit-advanced.md`).
A8. **« Mettre à l'arrière-plan » fusionne les flux de contenu de la page** (lopdf) avant de réordonner :
    PDFium dessine flux par flux. Le PDF reste valide ; les objets dupliqués sont des XObjects (non
    éditables comme texte).
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
A15. **Rail droit en fenêtre basse** (moins de ~650 px de haut) : les groupes du haut et du bas du rail
     peuvent se chevaucher (antérieur au hub, non traité).
A16. **Aide de la CLI pour `stamp` et `page-numbers`** : clap remplace toute séquence `{n}` d'un texte d'aide
     par un saut de ligne (`clap_builder` 4.6, sans échappement possible) ; l'aide affiche donc « : ⏎ = page
     number » au lieu de `{n}`. Les variables `{n}` et `{total}` fonctionnent ; seul le texte d'aide est
     tronqué (relevé par la revue du hub, non traité).
A17. **Indicateur « modifié » après annulation** : annuler jusqu'à l'état d'origine laisse le document marqué
     « modifié » (l'historique ne mémorise pas le point d'enregistrement ; relevé par la revue du hub, non
     traité).
