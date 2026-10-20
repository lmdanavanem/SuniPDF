# SuniPDF 0.34.0

[English](RELEASE_0.34.0.md)

## Modifications

- Inspecter le PDF enregistré depuis les propriétés ou la nouvelle commande JSON `inspect`. Sept
  inventaires bornés repèrent JavaScript, actions automatiques/externes, fichiers embarqués, XFA,
  multimédia et déclarations de signatures, sans exécution, exposition des contenus ni modification.
- Conserver les résultats partiels avec un état explicite et le code CLI `2`. Prendre en charge les
  sources de mot de passe existantes et les fichiers chiffrés. Afficher le SHA256 de la version examinée.
- Refuser les réponses UI périmées après modification, changement d’onglet ou fermeture. Exiger un
  enregistrement préalable ; ne pas présenter un brouillon ou un instantané PDFium comme le fichier sauvé.
- Calculer par blocs l’empreinte partagée avec les signatures, sans second tampon complet ; refuser une
  taille ou empreinte différente et conserver les octets capturés comme source de l’opération.
- Ajouter des PDF/xref écrits indépendamment et quatorze scénarios sur la CLI empaquetée. Ajouter les
  ZIP/MSI publics 0.33.0 aux références de migration avec empreintes et propriétés vérifiées.

[Utilisation et bornes](PDF_INSPECTION.fr.md). Aucune nouvelle dépendance d’exécution. Anglais par défaut,
catalogue français équivalent.

## Validation

[0.34.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.34.0) sous `lmdanavanem`, source `521618f4aab6cd17f7f829bc739bb3f7a93c9c5e`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342371499) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342385646) réussies : **600 tests, zéro échec**, 10 ignorés par défaut, **1108 clés EN/FR**, **14 scénarios d’inspection empaquetée**, **26 scénarios portables**, **20 références MSI/100 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Les contrôles qpdf, lecteurs indépendants, corpus PDF, mutations et Office existants restent inclus.

Six fichiers publics et copies `dist/` vérifiés par SHA256. Le harnais conservé de la 0.32.0, dont le module du metteur à jour est inchangé et vérifié, a téléchargé et contrôlé les **26,741,242 octets** du ZIP x64, sans installation sur le poste.

Inventaire structurel sans garantie d’innocuité ni validation des signatures/PDF/A/PDF/UA. Paquets non signés ; validations natives/matérielles et critères 1.0 encore ouverts. Projects reste reporté à la demande du propriétaire ; aucune synchronisation nouvelle annoncée.

[Preuves détaillées](qa/RELEASE_0.34.0.json).

## Limites

L’inventaire ne constitue ni une garantie d’innocuité, ni un antivirus, ni une vérification de signature
ou de conformité. Interface native/Narrator, matériel imprimante/GPU/ARM64, authenticité indépendante des
mises à jour, signature de code, corpus hostile élargi, fuzzing guidé et lecteurs tiers restent ouverts
selon les [critères 1.0](RELEASE_1.0_CRITERIA.fr.md). Projects reste reporté à la demande du propriétaire.
