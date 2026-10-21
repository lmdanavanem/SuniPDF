# Comptage sur plans et tableau des annotations

[English](MARKUP_REGISTER.md) · [Mesures](MEASUREMENTS.fr.md) · [Backlog AEC](AEC_BACKLOG.fr.md)

Le lot local 0.41.0 ajoute des comptages persistants et un tableau de toutes les annotations. Ces outils restent gratuits, open source et hors ligne. Cloud et collaboration sont reportés.

## Compter les éléments

1. Ouvrez **Commenter → Comptage manuel…**, indiquez un groupe d’équipements/types et une taille de 8 à 72 points PDF, puis **Commencer le comptage**.
2. Cliquez au centre de chaque élément. Chaque clic crée une annotation PDF Stamp, représentée par un cercle bleu et une croix. Le repère complet doit tenir dans la page. Un glissement de plus de deux points ne crée pas de repère.
3. Déplacez le repère avec **Sélectionner une annotation**, ou supprimez-le depuis les commentaires. Annuler/Rétablir restaure ces changements.
4. Le **Tableau des annotations** affiche les totaux par groupe. **Modifier les champs** peut changer le groupe sans changer identité, apparence ou position. Une étape Annuler restaure le groupe précédent.

Chaque repère représente un élément ; aucun total saisi séparément ne peut diverger des annotations. Enregistrement/réouverture, rotations et décalages CropBox sont conservés. Des identités `/NM` dupliquées refusent l’export. Réponses et états de relecture PDF restent disponibles. Les repères masqués restent dans les totaux et portent leur indicateur de masquage dans les exports.

Seuls les repères manuels SuniPDF reconnus sont comptés. Les tampons génériques importés et les propositions de recherche visuelle restent non quantifiés. Reconnaissance de symboles, comptage automatique et import propriétaire Bluebeam ne sont pas implémentés.

## Examiner les annotations

**Commenter → Tableau des annotations** lit toutes les pages sur le worker documentaire. Colonnes : page/indice, type, auteur, date PDF, état PDF, statut métier, discipline, groupe et commentaire. **Inclure réponses et états de relecture** ajoute les lignes enfants.

- Recherche sans distinction de casse, y compris les noms/valeurs personnalisés. Les filtres nommés exigent un texte exact ; le filtre de date recherche dans la date PDF, par exemple `D:202610`.
- Tri par page, auteur, date, type, état PDF, statut métier, discipline ou groupe, dans les deux sens. Les dates PDF sont triées lexicalement, sans normalisation des fuseaux.
- **Voir sur la page** ferme le tableau, rejoint le rectangle et sélectionne l’annotation dans les commentaires. Aucun bouton de positionnement si le rectangle est inconnu.
- **Modifier les champs** remplace statut métier, discipline et jusqu’à 16 champs texte, un `NOM=VALEUR` par ligne. Les champs optionnels vides sont effacés ; les noms dupliqués sont refusés. Un repère expose aussi son groupe. Chaque application constitue une étape Annuler.
- Un statut métier comme `Prêt pour chantier` ne remplace pas l’état PDF standard `Accepted`. Les champs sont du texte ; aucune formule n’est exécutée. Colonnes typées et tri par clé personnalisée restent AEC-004/007.

Le tableau affiche 50 lignes à la fois ; l’export inclut **toutes les lignes filtrées**. Actualiser relit l’instantané courant. Les modifications vérifient révision et identité pour refuser les indices décalés ou périmés. Un échec conserve la saisie et n’ajoute pas d’étape réussie à l’historique.

## Exporter

Le JSON `sunipdf-annotation-register`, version 1, conserve SHA256 de l’instantané courant en mémoire, pages, filtres/tri, lignes et totaux. Chaque total référence ses repères par page physique, indice à partir de zéro et `/NM`. Les inconnus restent `null`. Rectangles en points affichés, origine en bas à gauche.

CSV contient une ligne par annotation, la révision source et éventuellement `quantity=1`. Une cellule vide représente une valeur inconnue ; JSON distingue null et texte vide. Toutes les cellules sont entre guillemets. Une apostrophe neutralise les préfixes de formule, y compris après espaces, contrôles ou BOM. Les champs personnalisés sont dans une cellule JSON. CSV ne crée pas de fausses annotations pour les sous-totaux ; les totaux et références sont dans JSON.

Aucun fichier existant ou original n’est remplacé, même après une confirmation système ou avec `--overwrite` : choisissez un nouveau chemin. Le tableau nécessite le droit de copie PDF ; ajouter des repères nécessite le droit d’annotation. AES-128/AES-256 est conservé, y compris lorsque la modification générale est interdite.

```powershell
sunipdf-cli count plan.pdf --group Luminaires --point 120,90 --page 1 --size 18 --out compte.pdf
sunipdf-cli annotation-register compte.pdf --count-group Luminaires
sunipdf-cli annotation-register compte.pdf --query A101 --sort author --descending --out tableau.json
sunipdf-cli annotation-register compte.pdf --format csv --out tableau.csv
sunipdf-cli annotation-fields compte.pdf --page 1 --index 0 --count-group Prises --business-status "Prêt pour chantier" --discipline Electricite --field Piece=A101 --out revu.pdf
```

Pages physiques à partir de un ; `--index` à partir de zéro dans `/Annots`, fourni par le tableau. `annotation-fields` remplace les champs et efface les champs optionnels omis. CSV exige `--out` ; JSON accepte aussi la sortie standard. Les options de mots de passe par environnement/stdin fonctionnent sur les trois commandes.

## Limites et preuves

Maximum 1000 pages, 10000 annotations au total, 4096 par page, 4 Mio de texte/métadonnées et 64 niveaux de réponses. Chaînes standards limitées à 16 Kio, données SuniPDF à 32 Kio. Groupe : 1 à 512 octets UTF-8. Maximum 16 champs personnalisés, noms de 64 octets et valeurs de 1024 octets. Les nouvelles valeurs métier/personnalisées refusent les caractères de contrôle.

Tableaux illisibles, données SuniPDF malformées/inconnues, réponses cycliques/profondes et identités ambiguës refusent le tableau complet, sans faux zéro ni total partiel. Le format SuniPDF est indépendant des extensions propriétaires.

La régression couvre quatre rotations/CropBox, PDFium/MuPDF, qpdf, identités, changements de groupe, filtres, Unicode, exports protégés, préservation des fichiers, AES et les deux mots de passe. Les tests moteur/worker couvrent cibles périmées, Annuler/Rétablir et suppression. Les preuves sont publiées avec la release ; des plans synthétiques ne prouvent pas une compatibilité universelle. [Les critères 1.0 restent ouverts](RELEASE_1.0_CRITERIA.md).
