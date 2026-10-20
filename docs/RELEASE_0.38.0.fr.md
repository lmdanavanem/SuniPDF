# SuniPDF 0.38.0

## Changements

- Six zones d’en-tête/pied modifiables dans Pages → Numéros de page et la commande `headers-footers`.
- Numéro absolu et total des pages, taille, marges, sélection et coordonnées tournées/recadrées.
- Refus atomique du dépassement, conservation du chiffrement autorisé et nouvelle copie en CLI.
- Régression indépendante étendue à 42 scénarios, 148 appels CLI et deux contrôles qpdf.

[Utilisation et limites](PAGE_DECORATIONS.fr.md).

## Validation

[0.38.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.38.0) sous `lmdanavanem`, source `82a231fe6e423023030e099780d9a55aeebac72f`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395347423) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39395359394) réussies : **623 tests, zéro échec**, 10 ignorés par défaut, **1143 clés EN/FR**, **30 scénarios portables**, **24 références MSI/120 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **42 scénarios, 148 appels CLI et 2 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,823,494 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée.

[Preuves détaillées](qa/RELEASE_0.38.0.json).

## Limites

Zones sur une ligne, colonnes égales et texte de premier plan. Le contenu existant peut chevaucher : inspecter avant d’enregistrer. Pas de date automatique, règles paires/impaires, qualification des écritures complexes ou certification de conformité. Paquets non signés et ARM64 uniquement compilé en croisé. Les autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Projects reste différé à la demande du propriétaire.
