# SuniPDF 0.40.0

## Modifications

Annotations locales de longueur, polyligne, périmètre, surface polygonale simple et angle : calibration explicite sur deux points, unités métriques/impériales décimales, précision et données JSON. Chaque mesure conserve son échelle et sa géométrie ; aperçu, Annuler et copies CLI préservent les originaux. Tampons Stamp avec image intégrée, proportions et transparence. Filigranes texte/numérotation : vérification de toutes les pages et refus atomique des marges invalides ou débordements. Paquets 0.39.0 ajoutés aux migrations. Backlog AEC de 26 cartes donnant priorité au local ; cloud et collaboration reportés à la demande du propriétaire, tarifs ultérieurs.

[Mesures](MEASUREMENTS.fr.md) · [Tampons et décorations](PAGE_DECORATIONS.fr.md) · [Backlog AEC](AEC_BACKLOG.fr.md).

## Validation

[0.40.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.40.0) sous `lmdanavanem`, source `5e2d2705411b2b82d7f03bf78c0217ce49f261b5`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39661526853) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39661547803) réussies : **637 tests, zéro échec**, 10 ignorés par défaut, **1163 clés EN/FR**, **32 scénarios portables**, **26 références MSI/130 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **80 scénarios, 256 appels CLI et 10 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **27,121,645 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée.

[Preuves détaillées](qa/RELEASE_0.40.0.json).

## Limites

- Saisie des sommets et de la calibration en coordonnées de page affichée dans les annotations enrichies. Maximum 128 sommets et 0 à 4 décimales ; polygones simples uniquement, pieds/pouces décimaux.
- Chaque annotation possède sa calibration. Recréez-la pour changer géométrie ou échelle ; déplacement, redimensionnement et modification générique du libellé sont refusés pour éviter des valeurs périmées. Échelle globale, accrochage, vues multi-échelles, volumes, reconnaissance et comptages automatiques restent à développer.
- Les données JSON suivent un format SuniPDF documenté. Compatibilité avec les métadonnées propriétaires Bluebeam et import universel non annoncés. Les critères AEC incomplets restent ouverts.
- Tampons image : 64 Mio encodés, 4096 pixels par côté et quatre millions de pixels. Pixels intégrés indépendants du fichier source après application ; aucune signature numérique ou vérification d’identité.
- Le local reste gratuit et open source. Cloud, collaboration, déploiement et facturation sont reportés ; tarifs non définis. Modifications Projects différées.
- Paquets Windows non signés faute de clé de publication indépendante ; ARM64 compilé uniquement. Matériel physique, impression, Narrateur, interaction avec le metteur à jour installé, corpus élargi et conformité PDF/A/PDF/UA restent des qualifications 1.0 distinctes.
