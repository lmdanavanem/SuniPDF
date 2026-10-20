# SuniPDF 0.36.0

## Changements

- Personnalisation des raccourcis dans Aide et la palette, avec filtre, refus des conflits, désactivation et restauration.
- Préférences atomiques ; Annuler abandonne le brouillon et un échec d’écriture conserve les réglages actifs. Les installations existantes gardent les valeurs par défaut.
- Combinaisons actives dans menus, accueil, infobulles et palette ; protection des variantes avec Shift et de la saisie.
- Paquets 0.35.0 ajoutés aux contrôles de mise à niveau ZIP/MSI conservés. Aucune dépendance applicative ajoutée.

[Utilisation et limites](KEYBOARD_SHORTCUTS.fr.md).

## Validation

[0.36.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.36.0) sous `lmdanavanem`, source `1c48cf33db3d77c728dd6ed556ff357d55a84bbb`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355377383) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39355387902) réussies : **613 tests, zéro échec**, 10 ignorés par défaut, **1128 clés EN/FR**, **28 scénarios portables**, **22 références MSI/110 journaux**. Sept nouveaux tests egui sans fenêtre contrôlent raccourcis, conflits, focus, affichage, annulation et compatibilité des préférences. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Contrôles qpdf, lecteurs indépendants, polices, rendu DX12 logiciel, corpus, mutations et Office conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les **20 scénarios d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, avec module de mise à jour inchangé vérifié, a téléchargé et contrôlé les **26,788,530 octets** du ZIP x64 sans installation sur le poste.

Claviers Windows natifs et Narrateur restent à qualifier. Explorer reste à faire ; seules les commandes de la table existante sont personnalisables. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects reportées à la demande du propriétaire, sans synchronisation nouvelle annoncée.

[Preuves détaillées](qa/RELEASE_0.36.0.json).

## Limites

Seules les commandes de la table existante sont personnalisables ; touches système, navigation, presse-papiers et édition de texte restent réservées. Explorer reste à faire. Paquets non signés et sans clé de publication indépendante ; ARM64 compilé sans exécution matérielle. Les autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Modifications Projects différées à la demande du propriétaire.
