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

11. **MSI non construit sur ce poste** : `winget install WiXToolset.WiXToolset` échoue sans élévation (exige
    .NET Framework 3.5 et des droits administrateur). Le script `package-msi.ps1` est prêt ; la CI `release.yml`
    utilise le WiX 3.14 préinstallé sur `windows-latest`. Alternative locale : installer WiX en session administrateur.
12. **Build ARM64 non vérifié** : `package-portable.ps1 -Arch arm64` compile pour `aarch64-pc-windows-msvc`
    (binaires PDFium ARM64 disponibles) mais n'a jamais été exécuté ici.

## Application

0. **Icônes Fluent** : lues dans `C:\Windows\Fonts\SegoeIcons.ttf` (ou `segmdl2.ttf`). Sur un Windows sans ces
   polices (cas rare : LTSC allégé), les icônes sont remplacées par des symboles texte, moins élégants mais lisibles.

6. **Pas encore d'icône ni de ressources de version Windows** (`.rc`) pour l'exécutable.
7. **Affichage page unique seulement** ; pas d'onglets, pas de cache de rendu : chaque changement de
   page ou de zoom relance un rendu complet (acceptable pour la tranche 1, voir P2).
8. **Rendu à fort zoom** limité à 16 384 px de côté (`MAX_RENDER_SIDE_PX`) ; au-delà, message d'erreur.
   Le rendu en tuiles (P2) lèvera cette limite.
9. **Chiffrement testé uniquement en RC4 40 bits** (fixture générée). AES-128/256 géré par PDFium mais
   sans fixture dans le dépôt : à ajouter (qpdf en CI, P2).
10. **La boîte de dialogue d'ouverture (`rfd`) est modale et bloque la boucle UI** le temps du choix :
    comportement natif Windows, acceptable.
