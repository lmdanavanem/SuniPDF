# SuniPDF 0.33.0

[English](RELEASE_0.33.0.md)

## Modifications

- Conserver les titres UTF-8 importés bornés contenant tabulations et retours à la ligne lors des opérations
  de pages/destination. Les titres créés ou renommés gardent le contrôle sur une seule ligne ; les sources
  et titres bruts restent inchangés lors de la création d’une copie distincte.
- Migrer egui/eframe ensemble vers la 0.37, avec les API de rôles d’accessibilité, identifiants stables et
  défilement prises en charge. Les coordonnées PDF restent bornées pour restaurer la position avec
  Précédent/Suivant après un changement de zoom de destination. Dependabot groupe désormais ces mises à jour.
- Actualiser les 17 références de licences du framework selon les paquets verrouillés. Les textes complets
  restent exigés pour les deux graphes Windows ; l’indisponibilité des objets VCS egui concernés est
  explicitée dans les [mentions des composants tiers](THIRD_PARTY_NOTICES.fr.md).
  Inclure la cinquième police source, le sous-ensemble `egui-icons` sous MIT, et sa notice de modification ;
  une police inconnue ou sans notice empêche toujours le paquetage.
- Ajouter sept exemples indépendants de la PDF Association, épinglés par révision et SHA256, sous licence
  CC BY-SA 4.0. Texte/pixels, annotations, qpdf et sources inchangées sont contrôlés pendant optimisation et
  extraction ; ces exemples pédagogiques sont distincts des quatre documents NIST.
- Ajouter 32 mutations PDF et 32 CMS avec résultats structurés, réouverture, sources inchangées, contrôle
  positif de signature et contrôles négatifs de durée/sortie. Les processus natifs ont des limites explicites
  de temps, de mémoire échantillonnée et de sortie par canal. Ce contrôle déterministe ne remplace pas le
  fuzzing guidé par la couverture.
- Enregistrer durée par commande, pic échantillonné de mémoire résidente et caractéristiques matérielles/CI
  dans les rapports. Ajouter ZIP/MSI publics 0.32.0 aux migrations réelles, avec empreintes et propriétés MSI vérifiées.
- Publier les [critères d’acceptation 1.0](RELEASE_1.0_CRITERIA.fr.md), garder les P1 de qualification ouverts
  et corriger les anciens statuts de distribution/pivot dans la documentation actuelle.

## Limites

Paquets Windows non signés et aucune clé indépendante de publication configurée. Les mises à jour reposent
encore sur HTTPS GitHub et le SHA256 du même canal. ARM64 est compilé sans exécution matérielle. Impression
physique, fenêtres/GPU réels, Narrator, dialogues natifs d’installation/mise à jour/sauvegarde, interopérabilité
Acrobat/Foxit, élargissement du corpus réel/hostile et fuzzing guidé restent à qualifier. Aucune certification
PDF/A/PDF/UA ou LTV d’archivage. Les PDF téléchargés et dérivés sont exclus de Git et des paquets.

## Validation et distribution

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.33.0) sous `lmdanavanem`, release `417802490`, source immuable `c5dcd31cd1b93b601b3e1d0e28a98e99400f48cb`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332569906) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39332607929) réussis : **592 tests**, zéro échec, 10 ignorés par défaut, **1092 clés EN/FR** ; 84 contrôles qpdf et 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, quatre documents NIST/744 pages ; **7 exemples PDF 2.0/8 pages/21 contrôles qpdf**, **64 mutations et 2 contrôles de garde**, **25 scénarios portables**, **19 références MSI/95 journaux**. La CLI empaquetée passe aussi les 7 exemples/21 contrôles qpdf, 11 scénarios d’ordre d’objets/3 contrôles qpdf, 11 d’analyse locale et 8 de sécurité. CodeQL Rust/Python/Actions réussit, sans alerte ouverte.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais conservé de la 0.32.0, dont le module du metteur à jour est inchangé et vérifié par empreinte, a téléchargé et vérifié le ZIP x64 de **26,726,762 octets en 1.65 s**, sans installation.

La mise à jour Projects est reportée à la demande explicite du propriétaire. Le dernier contrôle public montre **43 cartes, 25 Done, 18 Todo** ; les deux P1 ont une livraison partielle documentée, sans clôture ni changement de statut annoncé.

[Preuves détaillées](qa/RELEASE_0.33.0.json).
