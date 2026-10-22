# SuniPDF 0.47.0 — superpositions de révisions alignées

[English](RELEASE_0.47.0.md) · [Guide de superposition](REVISION_OVERLAY.fr.md)

Statut : **en développement**. Ce périmètre est en cours d’implémentation ; il n’est pas encore qualifié ni publié.

## Périmètre

**Superposition de révisions** s’ouvre depuis **Tous les outils** ou **Comparer des fichiers**, sans document ouvert obligatoire. Sélectionnez une page physique dans chacun de deux PDF locaux enregistrés. La base apparaît en magenta et la révision en cyan, avec une opacité de révision réglable. Les modifications non enregistrées dans les onglets ne sont pas incluses.

Alignez la révision par translation, rotation horaire et facteur d’échelle uniforme explicites, ou cliquez deux points correspondants sur chaque image source. Relancez l’aperçu après modification des réglages ; la réinitialisation efface l’alignement. Les coordonnées sont des points PDF affichés depuis le coin supérieur gauche, après prise en compte du recadrage et de la rotation de page. La page de base définit le cadre de sortie ; le contenu révisé qui en dépasse est coupé.

Chaque PDF est limité à 64 Mio. Chaque image source est bornée indépendamment, avec un côté maximal de 1536 pixels par défaut et une limite supérieure de 2048. Le traitement prend en charge l’annulation et ne conserve aucun mot de passe dans l’aperçu préparé. Les deux PDF doivent autoriser la copie ; des mots de passe distincts sont pris en charge.

L’interface et la CLI exportent un nouveau PNG ou rapport JSON. Le rapport consigne les empreintes des deux sources, les pages choisies, leur géométrie, l’alignement et les paramètres de rendu. L’export compare à nouveau les fichiers enregistrés à leurs empreintes SHA-256 et refuse les destinations existantes. Aucun des deux PDF n’est modifié.

## Qualification

Les résultats moteur, interface, CLI, lecteurs PDF indépendants, paquets et téléchargements publics seront consignés après exécution sur la source de publication. Aucun nombre de tests réussis ni fichier publié n’est annoncé ici.

## Limites restantes

Ce lot superpose les images de deux pages explicitement choisies. L’alignement est manuel ou déterminé par deux paires de points ; il ne comprend ni recalage automatique, ni comparaison vectorielle, ni OCR, ni PDF fusionné, ni contour automatique des changements. Agrandir une image ne restitue pas les détails perdus au rendu. La superposition ne modifie ni mesures, ni calibration de page, ni contenu PDF. AEC-013 reste partiel ; AEC-014 reste ouvert. Qualification native/matérielle et critères 1.0 restent distincts.
