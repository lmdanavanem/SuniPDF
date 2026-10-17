# SuniPDF 0.30.0

[English](RELEASE_0.30.0.md)

Cette version couvre cinq Bugs et quatorze éléments P2 Roadmap. Les entrées prises en charge et les
limites sont décrites ci-dessous ; les validations exécutées et les téléchargements publics vérifiés
sont consignés dans [l’état du projet](PROJECT_STATUS.fr.md).

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

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0) sous `lmdanavanem`, release `415568680`, source immuable `4db31c0ee62cfae9d65239edf1eac93db147841e`. Les cinq Bugs et quatorze P2 Roadmap sont livrés dans leurs [limites documentées](RELEASE_0.30.0.fr.md). P1 Roadmap et Suggestions restent différés.

[Workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) et [CI main](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) réussis sur cette source : **551 tests**, zéro échec, 9 ignorés par défaut ; **1047 clés EN/FR**, **84 contrôles qpdf** plus **9 sorties de fonctions indépendantes**, **4 cas de polices MuPDF**, DX12 logiciel WARP, **14 allers-retours Office**, 4 PDF licenciés/744 pages, **21 scénarios portables**, **15 bases MSI/75 journaux**. CodeQL Rust, Python et Actions réussis ; zéro alerte ouverte.

Six fichiers publics vérifiés octet par octet avec les paquets préparés et leurs SHA256. Le metteur à jour de production a vérifié le ZIP x64 de 26,978,019 octets en 2.55 s, sans installation sur cet hôte, au moyen du banc de test déjà compilé dont le module de mise à jour est inchangé. Projet relu : **42 éléments uniques, 20 Todo, 0 In Progress, 22 Done**, trois filtres enregistrés et aucune modification non enregistrée. [Preuves et limites](PROJECT_STATUS.fr.md).

Les paquets restent non signés. ARM64 est compilé et empaqueté, sans exécution sur matériel ARM64. Imprimantes physiques, Narrateur, interactions manuelles de mise à jour et GPU/fenêtres physiques restent à qualifier ; WARP prouve uniquement le rendu logiciel hors écran. Les entrées non prises en charge restent refusées. La conservation PDF/A ne certifie pas PDF/A/PDF/UA.

Consulter les notices des [polices](FONT_EMBEDDING.fr.md), [annotations](RICH_ANNOTATIONS.fr.md), [caviardage](REDACTION.fr.md), [signets et structures](BOOKMARKS_AND_PAGE_STRUCTURE.fr.md), [conversions Office](OFFICE_CONVERSION.fr.md), [signatures](SIGNATURE_SECURITY.fr.md) et de [l’assainissement](SECURITY_MODEL.fr.md).
