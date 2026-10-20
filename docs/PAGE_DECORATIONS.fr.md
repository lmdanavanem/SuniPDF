# Décorations de page

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
