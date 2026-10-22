# Commandes de lecture et outils

[English](READER_WORKSPACE.md) · [Guide utilisateur](USER_GUIDE.fr.md)

Ce guide décrit la version 0.50.0. Fichiers publiés et qualification figurent sur la [page de version 0.50.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.50.0).

## Trouver un outil

Choisissez **Outils** dans la barre principale, ou appuyez sur **F9**. Le volet gauche passe entre **Outils**, **Pages** et **Signets**, avec un seul panneau à la fois. Sa croix le masque ; la barre principale ou les commandes de navigation permettent de le rouvrir.

Le panneau regroupe 18 outils en cinq familles :

| Famille | Outils |
|---|---|
| Réviser et mesurer | Commenter, Mesures, Table des quantités, Bibliothèque d’annotations, Superposition de révisions, Ajouter un tampon |
| Modifier et organiser | Modifier un PDF, Organiser les pages, Caviarder un PDF |
| Formulaires et signatures | Remplir et signer, Préparer un formulaire, Utiliser un certificat |
| Créer et convertir | Créer un PDF, Combiner des fichiers, Exporter un PDF, Numériser et OCR, Compresser un PDF |
| Sécurité | Protéger un PDF |

**Rechercher un outil** filtre les noms, descriptions et familles dans la langue active. Tous les mots saisis doivent correspondre. **Effacer la recherche** rétablit la liste. Un outil nécessitant un PDF propose d’en ouvrir un si besoin. La palette de commandes reste accessible avec **Ctrl+K**.

Ouvrez les mesures par **Outils → Réviser et mesurer → Mesures**. Cette famille contient aussi **Bibliothèque d’annotations**, **Table des quantités** et **Superposition de révisions**. Mesures et bibliothèque n’ont plus de boutons distincts dans la barre principale.

## Mesurer avec le panneau latéral

Choisir **Mesures** ouvre ses réglages à droite, à côté du PDF. Quatre sections regroupent les commandes :

- **Créer une mesure** : choisir type, unité et nombre de décimales.
- **Échelle de la page** : calibrer la page courante et gérer son échelle temporaire ou enregistrée.
- **Tracé** : lire les instructions, annuler un point, prévisualiser ou abandonner un brouillon.
- **Données enregistrées** : consulter les mesures enregistrées et ouvrir la table des quantités.

Faites défiler le panneau si nécessaire. Ces commandes occupent le panneau latéral plutôt qu’une barre d’espace de travail horizontale supplémentaire. Choisissez **Mesures enregistrées** dans **Données enregistrées** pour consulter ou modifier un enregistrement pris en charge. Consultez les [mesures calibrées](MEASUREMENTS.fr.md) et la [modification des géométries enregistrées](MEASUREMENT_GEOMETRY.fr.md) pour les limites de calcul et d’édition.

Le côté droit affiche un seul panneau contextuel à la fois, notamment Commentaires, Objets, formulaires ou réglages de mesure. Pendant un brouillon de mesure, l’ouverture d’un autre panneau contextuel est bloquée avec une invitation à terminer ou annuler le brouillon. Quitter explicitement l’outil de mesure ou changer d’outil annule son brouillon local ; les échelles de page confirmées sont conservées.

## Commandes de page et de zoom

Les commandes de lecture se trouvent sous le PDF. Utilisez les boutons précédent/suivant ou saisissez un numéro de page ou un libellé pris en charge, puis appuyez sur **Entrée**. Le nombre total de pages figure à côté du champ.

Utilisez **Zoom arrière**, la liste de zoom ou **Zoom avant**. Le pourcentage, y compris une valeur intermédiaire comme **149%**, reste sur une ligne. La liste propose des valeurs de 25 à 1600 % ; **Page** et **Largeur** ajustent la page ou la double page affichée. Les raccourcis clavier et le zoom à la molette restent disponibles.

**Options de lecture** contient Page unique, Défilement continu, Deux pages, Vue livre, les rotations de lecture dans les deux sens et Lecture plein écran. Les dispositions larges présentent aussi des boutons directs de rotation horaire et de plein écran. Dans une fenêtre plus étroite, les groupes de commandes passent sur une autre ligne et le menu conserve ces actions.

La rotation de lecture change uniquement l’orientation de l’affichage. Pour modifier la rotation des pages du PDF, utilisez **Organiser les pages**, puis enregistrez le document. Le rail d’icônes droit ouvre Pages, Signets, Commentaires, Objets et Rechercher ; la navigation de page et le zoom passent par les commandes du bas. La zone d’état affiche la disponibilité ou les messages d’opération.

## Périmètre

Cette version réorganise les opérations existantes de lecture, navigation et mesure. Elle n’ajoute ni accrochage vectoriel, ni recalcul par lots, ni nouveaux types de mesure. Les permissions PDF et les gardes des brouillons et aperçus restent applicables.

Le rendu hors écran et les interactions automatisées peuvent vérifier les dispositions testées. Ils ne constituent pas une qualification en fenêtre native, sur périphérique physique, avec lecteur d’écran ou sur matériel ARM64. Les preuves de publication précisent les contrôles réellement exécutés.
