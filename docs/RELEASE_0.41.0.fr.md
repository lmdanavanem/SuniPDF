# SuniPDF 0.41.0 — comptage sur plans et tableau des annotations

## Nouveautés

Repères de comptage persistants placés par clic sur les plans PDF locaux, regroupement, déplacement/suppression avec Annuler, changement de groupe sans changer l’identité et export des totaux reliés aux repères. Le tableau de toutes les annotations ajoute recherche, filtres nommés, tri, navigation, statuts métier, disciplines et champs texte personnalisés, avec exports CSV/JSON protégés. Les changements refusent les cibles périmées et préservent originaux et permissions. Pillow 13.0.0 remplace la dépendance vulnérable réservée aux tests. Les paquets 0.40.0 rejoignent les migrations ; cloud et collaboration restent reportés. Les écritures communes vérifient le droit PDF d’annotation pour déplacements, suppressions, réponses et autres modifications ; l’export refuse aussi les inventaires de pages incomplets et les identités de comptage malformées.

## Validation

[0.41.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.41.0) sous `lmdanavanem`, source `8e36a4bf2d2396e6cff92cea4fd006c8c964c62f`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39681919685) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39681938796) réussies : **643 tests, zéro échec**, 10 ignorés par défaut, **1198 clés EN/FR**, **33 scénarios portables**, **27 références MSI/135 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **100 scénarios, 339 appels CLI et 14 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés. 6 alertes de mots de passe fictifs ont été examinées et classées « used in tests » sous `lmdanavanem` ; analyse de sécurité maintenue.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **27,323,650 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée.

[Preuves détaillées](qa/RELEASE_0.41.0.json).

## Limites

- Comptages manuels explicites : un élément par repère SuniPDF. Reconnaissance de symboles, comptage automatique et import des métadonnées de comptage propriétaires Bluebeam non annoncés.
- Tableau limité à 1000 pages, 10000 annotations au total, 4096 par page, 4 Mio de métadonnées et 64 niveaux de réponses. Métadonnées SuniPDF inconnues ou malformées : refus de l’export complet. La consultation et l’export du tableau nécessitent le droit de copie PDF.
- Filtres nommés par texte exact ; recherche dans les champs personnalisés. Dates PDF conservées et triées lexicalement, sans normalisation des fuseaux. Champs personnalisés texte uniquement, maximum 16 ; colonnes typées, formules et tri par clé personnalisée restent à développer.
- Les exports décrivent l’instantané PDF courant en mémoire. JSON conserve valeurs nulles et liens vers les repères ; CSV utilise des cellules vides pour les inconnus et protège les préfixes de formule. Aucun fichier de sortie existant ou original n’est remplacé.
- Les fonctions locales restent gratuites et open source. Cloud, collaboration, déploiement, prix et modifications Projects restent reportés.
- Paquets non signés faute de clé de publication indépendante ; ARM64 compilé uniquement. Matériel, impression physique, Narrateur, interaction de mise à jour installée, corpus élargi et conformité PDF/A/PDF/UA restent des qualifications 1.0 distinctes.
