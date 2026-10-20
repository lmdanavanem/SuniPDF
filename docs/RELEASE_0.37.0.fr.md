# SuniPDF 0.37.0

## Changements

- Filigranes image dans la fenêtre Filigrane et la commande CLI `image-watermark`.
- Transparence, largeur proportionnelle, angle, sept positions et ajustement à la page affichée.
- Modification atomique des pages choisies, une annulation dans l’application, nouvelle copie en CLI et conservation du chiffrement autorisé.
- Régression indépendante sur géométrie, pixels, refus et protection avec qpdf ; paquets 0.36.0 ajoutés aux migrations.

[Utilisation et limites](PAGE_DECORATIONS.fr.md).

## Validation

[0.37.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.37.0) sous `lmdanavanem`, source `e206f6b8d7e7fe61101005ad86e1d26c40a8e448`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387964703) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39387976826) réussies : **618 tests, zéro échec**, 10 ignorés par défaut, **1134 clés EN/FR**, **29 scénarios portables**, **23 références MSI/115 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Le contrôle des décorations passe sur **24 scénarios, 113 appels CLI et 1 contrôle(s) qpdf**. Lecteurs indépendants, corpus, rendu logiciel, Office, mutations et contrôles de sécurité conservés. Deux alertes de mots de passe fictifs ont été examinées et classées « used in tests » sous `lmdanavanem` ; analyse de sécurité maintenue.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les scénarios de décorations, **20 d’image** et **14 d’inspection PDF**, ainsi que les suites conservées, ont été rejoués sur la CLI téléchargée. Le harnais conservé de la 0.32.0, dont le module de mise à jour inchangé a été vérifié, a téléchargé et contrôlé les **26,806,746 octets** du ZIP x64 sans installation sur le poste.

Interaction native Windows, claviers et Narrateur restent à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Modifications Projects différées à la demande du propriétaire ; aucune synchronisation nouvelle annoncée.

[Preuves détaillées](qa/RELEASE_0.37.0.json).

## Limites

Le filigrane image est un contenu de premier plan modifiable. Aucun arrière-plan, mosaïque, DRM ou contrôle de signature n’est promis. Les paquets restent non signés ; ARM64 est uniquement compilé en croisé. Les autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Projects demeure différé à la demande du propriétaire.
