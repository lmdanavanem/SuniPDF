# Ordre des objets de page

[English](OBJECT_ORDER.md)

Implémenté pour la 0.32.0 ; qualification consignée dans [État du projet](PROJECT_STATUS.fr.md).

## Bureau

Ouvrez le panneau Objets avec F3. Le haut de la liste correspond au premier plan. Glissez une ligne au-dessus ou au-dessous d’une autre ; le trait bleu indique la destination. Glisser une ligne sélectionnée déplace toute la sélection en un bloc, dans son ordre relatif d’origine. Glisser une ligne non sélectionnée déplace cet objet uniquement.

Échap annule. Un dépôt hors d’une ligne valide, sur une ligne sélectionnée ou à la même position ne fait rien. Un dépôt dans une autre page/document ou après une modification intermédiaire est refusé. La sélection suit les objets déplacés. Les boutons avancer/reculer et commandes clavier existants restent disponibles ; un déplacement terminé utilise une seule étape Annuler et le Rétablir de la transaction existante.

## CLI

Listez les indices avec `sunipdf-cli objects input.pdf --page 1`, puis :

```powershell
sunipdf-cli reorder-objects input.pdf --page 1 --objects "3,1" --at 0 --out output.pdf
```

Les pages commencent à un. Les indices d’objet et `--at` commencent à zéro dans l’ordre de dessin ; zéro est l’arrière-plan. `--at` désigne l’indice final du premier objet déplacé après retrait des objets sélectionnés. Les indices sont triés et dédoublonnés ; l’ordre relatif est conservé. Le JSON renvoie `new_indices` dans cet ordre. La destination maximale est le nombre d’objets moins le nombre d’indices distincts sélectionnés.

Les règles existantes de sortie/écrasement, sauvegarde atomique et mots de passe par environnement/stdin s’appliquent. Les indices, destinations ou pages invalides ne produisent pas de sortie. La permission effective de modification est contrôlée avant tout changement, y compris pour les opérations existantes premier/arrière-plan.

## Périmètre

Cette fonction change l’ordre du contenu de premier niveau : texte, tracés, images et Form XObjects entiers. Les annotations/champs superposés et objets internes aux Form sont séparés. Géométrie, styles et contenu restent associés aux objets ; les recouvrements visibles changent comme demandé. Une copie séparée préserve les sources, le chiffrement et les permissions. Comme les autres retouches de contenu, cette opération peut invalider les signatures numériques existantes ; enregistrez une copie. L’édition générale des calques OCG et les qualifications natives d’accessibilité/matériel restent séparées.
