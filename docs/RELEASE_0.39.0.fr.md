# SuniPDF 0.39.0

## Changements

- Modèles locaux nommés pour filigranes texte/image, numéros et six zones d’en-tête/pied.
- Charger un brouillon, enregistrer sous un nom unique ou supprimer explicitement ; 16 modèles au maximum.
- Préférences atomiques, chargement borné et isolation des modèles mal formés.
- Portée des pages à vérifier à chaque usage ; régressions de paquets, sécurité et migrations conservées.
- Vérification de version Office sans interface avec un budget annulable de 30 secondes au démarrage à froid ; limites de conversion et gardes de sécurité conservées.

[Utilisation et limites](PAGE_DECORATIONS.fr.md).

## Validation

[0.39.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.39.0) sous `lmdanavanem`, source `24e3dcd0978fd74fb63d1c43aa8e668f5774fe04`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404372880) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39404384844) réussies : **628 tests, zéro échec**, 10 ignorés par défaut, **1149 clés EN/FR**, **31 scénarios portables**, **25 références MSI/125 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **42 scénarios, 148 appels CLI et 2 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,879,736 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée.

[Preuves détaillées](qa/RELEASE_0.39.0.json).

## Limites

Les images référencent leur chemin d’origine : garder le fichier disponible. Vérifier la portée des pages. Enregistrer/supprimer un modèle persiste indépendamment de l’annulation du document. Pas de macro, import/export CLI ou stockage d’identité de signature. Paquets non signés et ARM64 uniquement compilé en croisé. Les autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Projects reste différé à la demande du propriétaire.
