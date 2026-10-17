# Signets et structure des pages

Le panneau des signets permet d’ajouter un signet principal ou enfant, de le renommer, de définir sa page, de réordonner des éléments frères et de supprimer une branche. Les modifications passent par Annuler/Rétablir. L’API permet aussi de changer le parent d’une branche. Limites : 4 096 signets, 32 niveaux et 1 024 octets UTF-8 par titre.

Les cycles, indices incorrects, profondeurs excessives et liens de structure incohérents sont refusés avant modification. Renommer ou déplacer conserve l’action, les coordonnées, le style et les métadonnées existants. Définir explicitement une nouvelle destination remplace l’action du signet concerné par une destination locale. Les actions non prises en charge restent des données PDF et ne sont jamais exécutées par l’éditeur.

La fusion, l’extraction et l’insertion conservent les libellés de pages existants et reconstruisent les signets, destinations locales, annotations, champs AcroForm sélectionnés, références de calques et pièces jointes nommées ou associées. Les destinations XYZ/Fit conservent leurs axes et valeurs nulles. Une destination vers une page exclue est retirée. Les pages et arbres de pages non sélectionnés ne sont pas accessibles au parcours de copie.

Une destination vers une page répétée désigne sa première occurrence dans la sortie. Les annotations ordinaires répétées reçoivent des propriétaires distincts. Répéter une page contenant des champs de formulaire est refusé. Les noms du document cible restent inchangés ; les collisions entrantes reçoivent un préfixe déterministe du type `S2:`. Les destinations locales sont remappées vers les pages, afin qu’un nom identique ne les redirige pas vers un autre document.

Les références de champs restent reliées à leurs widgets. Les collisions de noms de ressources de police utilisent des alias et les apparences par défaut sont adaptées. Les collisions de champs avec scripts ou calculs, XFA, collisions de ressources non typographiques et configurations de calques complexes sont refusées plutôt que dégradées silencieusement. Les groupes de calques simples, leur ordre et leurs états activé/désactivé sont conservés avec les références de ressources des pages.

Il s’agit d’une préservation bornée, sans garantie universelle pour toutes les extensions PDF ou procédures actives. Les arbres de noms, signets, héritages de pages et champs ont des limites de taille, profondeur et cycles. Une référence manquante ou un graphe non représentable entraîne un échec atomique. Les originaux restent inchangés. L’extraction conserve le chiffrement et les permissions ; une source chiffrée sans droit de copie ne peut pas être importée. L’insertion ne valide le document candidat qu’après réussite des contrôles.

Les tests inspectent indépendamment les destinations, valeurs et propriétaires des champs, identités des ressources de calques, octets des pièces jointes et absence des pages exclues. Ils couvrent AES-128/AES-256, les cycles malformés, champs dupliqués, restrictions de copie, originaux inchangés et restauration d’instantané. Un test du thread documentaire vérifie Annuler/Rétablir et l’historique après refus. Les résultats d’exécution doivent figurer dans les preuves de publication.

La CLI conserve `bookmarks` pour la lecture et ajoute `bookmark-add`, `bookmark-rename`, `bookmark-destination`, `bookmark-move` et `bookmark-delete`. Les indices de signets, pages et positions commencent à 1. `--parent` omis désigne la racine ; la position de déplacement est calculée après retrait de l’élément. Chaque commande écrit une nouvelle copie avec `--out`, sans remplacer l’entrée ou un fichier existant, même avec `--overwrite`.

Détails et syntaxe : [notice anglaise](BOOKMARKS_AND_PAGE_STRUCTURE.md).
