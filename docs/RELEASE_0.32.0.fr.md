# SuniPDF 0.32.0

## Nouveautés et corrections

- Réorganisation d’un objet ou de toute la sélection par glisser-déposer dans le panneau Objets, trait d’insertion, refus des déplacements obsolètes et annulation par Échap.
- Commande JSON `reorder-objects` : destination finale explicite, ordre relatif, sauvegarde et mots de passe existants.
- Contrôle de permission de modification avant changement de l’ordre ; les destinations invalides et opérations identiques sont refusées/ignorées avant mutation.
- La version publique 0.31.0 rejoint les régressions de mise à jour portable et MSI.

[Périmètre et utilisation](OBJECT_ORDER.fr.md).

Paquets Windows non signés ; aucune clé indépendante de publication disponible. ARM64 est compilé sans exécution matérielle. Les vérifications natives, imprimantes, lecteurs tiers et certifications PDF/A/PDF/UA restent séparées.

## Validation et distribution

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.32.0) sous `lmdanavanem`, release `417623635`, source immuable `8c492db35f1712648d5472593146077230befb7c`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306550382) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39306651110) réussis : **588 tests**, zéro échec, 9 ignorés par défaut ; **1092 clés EN/FR**, 84 contrôles qpdf + 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, 4 PDF sous licence/744 pages, **24 scénarios portables**, **18 références MSI/90 journaux**. CodeQL Rust/Python/Actions réussit sans alerte ouverte. La CLI empaquetée passe **11 scénarios d’ordre d’objets et 3 contrôles qpdf supplémentaires**, plus 11 scénarios d’analyse locale et 8 de sécurité conservés.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais fraîchement compilé sur cette source a téléchargé et vérifié le ZIP x64 de **27,119,935 octets en 1.75 s**, sans installation. Projects : **43 cartes, 25 Done, 18 Todo, zéro In Progress** ; Suggestion P3 livrée, critères conservés, description relue après rechargement et trois filtres enregistrés vérifiés.

[Preuves détaillées](qa/RELEASE_0.32.0.json).
