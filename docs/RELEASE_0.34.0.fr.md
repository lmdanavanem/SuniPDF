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

Qualification de publication en cours. Les résultats exacts de CI principale/tag, CodeQL, paquets,
téléchargements publics et metteur à jour seront consignés après vérification ; la stable reste 0.33.0.

## Limites

L’inventaire ne constitue ni une garantie d’innocuité, ni un antivirus, ni une vérification de signature
ou de conformité. Interface native/Narrator, matériel imprimante/GPU/ARM64, authenticité indépendante des
mises à jour, signature de code, corpus hostile élargi, fuzzing guidé et lecteurs tiers restent ouverts
selon les [critères 1.0](RELEASE_1.0_CRITERIA.fr.md). Projects reste reporté à la demande du propriétaire.
