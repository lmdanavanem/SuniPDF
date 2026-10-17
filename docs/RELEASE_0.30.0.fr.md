# SuniPDF 0.30.0

[English](RELEASE_0.30.0.md)

Version candidate : développement et validation en cours. La version publique stable reste la
[0.21.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.1) tant que les contrôles de publication
ne sont pas tous réussis.

## Édition et annotations

- Prévisualisation du réagencement de paragraphes multilignes avec les métriques réelles des polices
  PDF. Un texte ou un état graphique incompatible est refusé sans perdre le document ni le brouillon.
- Collage de contenu PDF sous forme d’objets individuellement modifiables ; le collage groupé reste
  disponible. Les graphes de ressources non pris en charge sont refusés.
- FreeText à police incorporée, polygones, polylignes, nuages, tampons et pièces jointes. La gomme
  supprime les traits complets touchés et conserve les traits intacts ainsi que les commentaires.
- Liens nommés et vues XYZ/Fit/FitH précises ; les axes et le zoom vides conservent la vue du lecteur.
  Les coordonnées suivent le rognage et la rotation de la page affichée.
- Pivot de rotation stable, remplacement explicite de l’opacité et mise à l’échelle facultative
  des annotations lors du redimensionnement du contenu des pages.

## Pages et protection des documents

- Création, renommage, déplacement, changement de destination et suppression de signets en interface
  et en ligne de commande.
- Conservation bornée des liens, champs, calques, pièces jointes et signets à la fusion, insertion et
  extraction ; refus atomique des structures malformées ou impossibles à préserver.
- Étiquettes sémantiques attachées au contenu des pages après réorganisation ; les doublons restent
  explicites et `#N` sélectionne toujours un numéro physique.
- Aperçu du caviardage avant application, marques standard persistantes non appliquées, couleur et
  texte de remplacement ; conservation du texte sélectionnable lorsque sa découpe est prouvée sûre.
- Copie assainie distincte : suppression des métadonnées cachées prises en charge, scripts, signets,
  destinations nommées, pièces jointes et ressources de polices inutilisées, avec réécriture complète
  et contrôle d’absence. Les anciennes copies, sauvegardes, récupérations et l’historique Annuler
  nécessitent leur propre nettoyage.

## Conversion locale et sécurité

- Conversion de fichiers Office compatibles sans macros par LibreOffice local déjà installé,
  traitements par lots annulables, nouvelles copies et profils Standard/Print réutilisables.
- Export du texte PDF simple vers DOCX, de lignes de texte alignées vers XLSX et de pages rendues vers
  PPTX. Les limites de fidélité sont mesurées et publiées.
- Séparation entre intégrité des signatures et confiance Windows, révocation et horodatage RFC3161.
  Le réseau exige un accord explicite ; les états hors ligne ou indisponibles restent visibles.
  Le signalement des données LTV incorporées ne vaut pas validation d’archivage.
- Stockage facultatif des images de signature protégé par DPAPI pour l’utilisateur Windows courant,
  avec migration contrôlée.
- Chemins Unicode des polices Windows, sous-ensembles de faces statiques TTF/OTF/TTC compatibles,
  réutilisation des programmes identiques et respect des restrictions d’incorporation.
- OpenGL reste le moteur par défaut ; wgpu est facultatif et dispose d’un contrôle DX12 logiciel natif.
- Inventaire des licences avec versions exactes et vérification du contenu réel des ZIP et MSI.

## Validation et distribution

La publication exige les contrôles x64 de formatage, Clippy et tests, la compilation ARM64, les lecteurs
indépendants de polices et de fonctions, qpdf et le corpus, les conversions Office mesurées, les mises à
jour portables et les transactions MSI. Le MSI x64 et les ZIP x64/ARM64 ont chacun une empreinte SHA256.

Les résultats exécutés et empreintes publiques seront consignés dans [l’état du projet](PROJECT_STATUS.fr.md).
Les paquets restent non signés ; matériel ARM64, imprimantes physiques et comportement manuel des fenêtres
et GPU natifs restent à qualifier séparément. Consulter les notices des
[polices](FONT_EMBEDDING.fr.md), [annotations](RICH_ANNOTATIONS.fr.md),
[signets et structures](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md), [conversions Office](OFFICE_CONVERSION.fr.md),
[signatures](SIGNATURE_SECURITY.fr.md) et de [l’assainissement](SECURITY_MODEL.fr.md).
