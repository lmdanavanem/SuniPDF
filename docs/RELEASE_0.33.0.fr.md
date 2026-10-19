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

## Validation

Qualification de publication en cours. Les résultats exacts de CI, paquets, téléchargements publics,
metteur à jour et Projects seront consignés après validation ; la version publique stable reste la 0.32.0.
