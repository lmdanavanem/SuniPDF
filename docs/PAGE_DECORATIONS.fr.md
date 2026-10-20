# Décorations de page

## Modèles réutilisables (0.39.0)

Dans Filigrane ou Numéros de page, choisir un modèle enregistré pour charger ses réglages. Modifier et vérifier le brouillon, puis cliquer sur le bouton habituel d’application pour modifier le PDF. Le choix du modèle ne change pas le document. Annuler abandonne l’opération en attente.

Pour conserver une configuration, saisir un nouveau nom puis cliquer sur **Enregistrer le modèle**. Choisir un modèle et cliquer sur **Supprimer le modèle** retire cette préférence. Ces boutons enregistrent immédiatement ; annuler ensuite la fenêtre du document ne revient pas sur un enregistrement/suppression explicite. Les noms sont uniques sans distinction de casse ; un nom existant est refusé, utiliser un nouveau nom ou supprimer d’abord l’ancien. Un échec d’écriture conserve les préférences actives.

Jusqu’à 16 modèles, avec un nom de 64 caractères, sont conservés dans le fichier local de préférences. Filigranes texte/image, numéros et six zones d’en-tête/pied sont pris en charge. La sélection des pages n’est pas enregistrée : vérifier toutes/page courante à chaque usage. Texte sur une ligne, 256 caractères par zone/modèle. Les modèles image gardent le chemin du fichier original : le conserver disponible. Le choix ne décode pas l’image ; l’application signale un fichier absent/invalide et préserve le document. Aucun contenu d’image ou identité de signature n’est stocké par cette fonction.

Les anciennes préférences gardent leurs valeurs par défaut ; les modèles mal formés sont ignorés sans réinitialiser les autres préférences valides. Un fichier de préférences dépassant 1 Mio est refusé au chargement et les valeurs par défaut sont utilisées. Pas d’import/export CLI de modèles, synchronisation cloud, macro ou coffre de mots de passe. Les contrôles CLI conservent les opérations image/en-tête ; les tests de préférences et dialogues sans fenêtre couvrent persistance et annulation. L’interaction Windows native reste à qualifier.

## En-têtes et pieds de page (0.38.0)

Ouvrir **Pages → Numéros de page**, puis choisir **En-têtes et pieds de page**. Renseigner une ou plusieurs des six zones : en-tête gauche/centre/droite et pied gauche/centre/droite. Choisir la taille, la marge et toutes les pages ou la page courante. `{n}` désigne le numéro absolu et `{total}` le nombre total de pages du document. Une annulation retire toute l’opération.

```powershell
sunipdf-cli headers-footers input.pdf --header-left "Report" --header-right "Internal" --footer-center "{n} / {total}" --pages 1-3 --size 10 --margin 28 --out decorated.pdf
```

Chaque zone occupe un tiers de la largeur utile, avec 8 points entre les colonnes. Le texte respecte la rotation et le recadrage affichés. Un texte dépassant sa colonne ou la moitié de la hauteur utile est refusé, sans réduction ni troncature silencieuse. Toutes les pages sélectionnées sont validées ensemble ; six objets texte modifiables au maximum sont ajoutés par page. Le contenu existant peut chevaucher ces zones : inspecter avant d’enregistrer.

Limites : 256 caractères sur une ligne par zone, aucun caractère de contrôle, taille de 4 à 72 points, marge de 0 à 1000 points et 1000 pages au maximum. Une zone non vide est obligatoire ; les zones vides sont omises. Les restrictions, le chiffrement autorisé, les mots de passe et la nouvelle copie en CLI suivent les mêmes gardes que ci-dessous. Pas de date automatique, compteur de section, règle première/paire/impaire ou balisage PDF. Les polices suivent le moteur texte existant ; les écritures complexes ne sont pas nouvellement qualifiées.

## Filigranes image (0.37.0)

Ouvrir **Pages → Filigrane**, choisir **Image** puis sélectionner un fichier PNG, JPEG, BMP, GIF, TIFF ou WebP. Régler la largeur en pourcentage de la page affichée, l’opacité, l’angle, la position et la marge. Appliquer à toutes les pages ou à la page courante. Une annulation retire toute l’opération. Enregistrer le PDF pour conserver le résultat.

Le filigrane est une image de premier plan modifiable ; sa transparence est multipliée par l’opacité choisie. Ses proportions sont conservées et l’image tient dans les deux dimensions de la page, rotation et recadrage compris. Le premier plan peut masquer du contenu : inspecter le résultat avant d’enregistrer. Le filigrane texte reste disponible.

```powershell
sunipdf-cli image-watermark input.pdf --image logo.png --pages 1-3 --width-percent 35 --opacity 0.25 --angle=-45 --position center --out marked.pdf
```

Positions : `center`, `top-left`, `top-center`, `top-right`, `bottom-left`, `bottom-center`, `bottom-right`. La numérotation commence à 1 ; les doublons ne sont appliqués qu’une fois. `--password-env NAME` et `--password-stdin` évitent les mots de passe dans les arguments. La commande écrit une nouvelle copie et refuse une destination existante ou le document original, même avec `--overwrite`.

Le décodage est local, orientation EXIF comprise. Limites : 64 Mio encodés, 4096 pixels par côté, quatre millions de pixels par image, 1000 pages et 64 millions de pixels d’image répartis sur les pages sélectionnées. Réglages invalides, images entièrement transparentes et documents interdisant la modification sont refusés. Le document en mémoire est remplacé uniquement si toutes les pages réussissent. Le chiffrement AES-128/AES-256 autorisé et les deux mots de passe sont conservés.

Les contrôles sur fichiers générés couvrent géométrie, pixels indépendants, protection, refus et qpdf. Ils ne qualifient pas le sélecteur natif, les claviers et Narrateur, les documents clients, l’impression physique ou la conformité PDF/A/PDF/UA. Pas de calque d’arrière-plan, mosaïque, animation, vérification de signature ou DRM.
