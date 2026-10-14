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
12. **Build ARM64 non vérifié** : `package-portable.ps1 -Arch arm64` compile pour `aarch64-pc-windows-msvc`
    (binaires PDFium ARM64 disponibles) mais n'a jamais été exécuté ici (voir `docs/plans/qa-hardening.md`).

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
