# Problèmes connus

## Environnement de build (ce poste)

1. **Pas de Visual Studio Build Tools ni de Windows SDK, pas de droits administrateur.** La cible MSVC
   de référence n'a donc jamais été compilée ici. Contournement : toolchain GNU + WinLibs (D008).
   À faire par l'utilisateur : installer « Build Tools pour Visual Studio » (charge de travail C++ Desktop),
   puis `rustup override unset` dans le dossier pour revenir à MSVC.
2. **Mémoire limitée (14 Go, ~1,5 Go libres pendant la session).** Avec 16 cœurs, `cargo` lance 16 `rustc`
   et le système atteint sa limite de commit : abandons `0xC0000409` et erreurs trompeuses
   (« only metadata stub found », ICE). Contournement : `cargo build -j 2` (ou `CARGO_BUILD_JOBS=2`)
   et `debug = "line-tables-only"` en profil dev.
3. **Les binutils livrés par rustup ne suffisent pas** pour les crates `raw-dylib` sur la cible GNU
   (`dlltool` sans `as`). WinLibs complet requis (voir `scripts/dev-env.ps1`).

## Dépendances

4. **wgpu/DX12 désactivé** : `wgpu-hal` 30.0.1 ne compile pas avec `gpu-allocator` 0.28 (D010).
   Backend glow/OpenGL utilisé. Impact possible : machines sans pilote OpenGL correct (VM, ARM64).
   Piste : `cargo update -p gpu-allocator --precise 0.27.x` ou eframe plus récent.
5. **pdfium-render 0.9.4 cible le build PDFium 7881** ; les builds plus récents (8100 au 2026-10-12)
   fonctionnent probablement mais ne sont pas testés. Version épinglée dans `scripts/fetch-pdfium.ps1`.

## Application

6. **Pas encore d'icône ni de ressources de version Windows** (`.rc`) pour l'exécutable.
7. **Affichage page unique seulement** ; pas d'onglets, pas de cache de rendu : chaque changement de
   page ou de zoom relance un rendu complet (acceptable pour la tranche 1, voir P2).
8. **Rendu à fort zoom** limité à 16 384 px de côté (`MAX_RENDER_SIDE_PX`) ; au-delà, message d'erreur.
   Le rendu en tuiles (P2) lèvera cette limite.
9. **Chiffrement testé uniquement en RC4 40 bits** (fixture générée). AES-128/256 géré par PDFium mais
   sans fixture dans le dépôt : à ajouter (qpdf en CI, P2).
10. **La boîte de dialogue d'ouverture (`rfd`) est modale et bloque la boucle UI** le temps du choix :
    comportement natif Windows, acceptable.
