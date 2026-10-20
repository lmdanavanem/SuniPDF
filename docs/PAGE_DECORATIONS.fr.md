# Filigranes image

Ouvrir **Pages → Filigrane**, choisir **Image** puis sélectionner un fichier PNG, JPEG, BMP, GIF, TIFF ou WebP. Régler la largeur en pourcentage de la page affichée, l’opacité, l’angle, la position et la marge. Appliquer à toutes les pages ou à la page courante. Une annulation retire toute l’opération. Enregistrer le PDF pour conserver le résultat.

Le filigrane est une image de premier plan modifiable ; sa transparence est multipliée par l’opacité choisie. Ses proportions sont conservées et l’image tient dans les deux dimensions de la page, rotation et recadrage compris. Le premier plan peut masquer du contenu : inspecter le résultat avant d’enregistrer. Le filigrane texte reste disponible.

```powershell
sunipdf-cli image-watermark input.pdf --image logo.png --pages 1-3 --width-percent 35 --opacity 0.25 --angle=-45 --position center --out marked.pdf
```

Positions : `center`, `top-left`, `top-center`, `top-right`, `bottom-left`, `bottom-center`, `bottom-right`. La numérotation commence à 1 ; les doublons ne sont appliqués qu’une fois. `--password-env NAME` et `--password-stdin` évitent les mots de passe dans les arguments. La commande écrit une nouvelle copie et refuse une destination existante ou le document original, même avec `--overwrite`.

Le décodage est local, orientation EXIF comprise. Limites : 64 Mio encodés, 4096 pixels par côté, quatre millions de pixels par image, 1000 pages et 64 millions de pixels d’image répartis sur les pages sélectionnées. Réglages invalides, images entièrement transparentes et documents interdisant la modification sont refusés. Le document en mémoire est remplacé uniquement si toutes les pages réussissent. Le chiffrement AES-128/AES-256 autorisé et les deux mots de passe sont conservés.

Les contrôles sur fichiers générés couvrent géométrie, pixels indépendants, protection, refus et qpdf. Ils ne qualifient pas le sélecteur natif, les claviers et Narrateur, les documents clients, l’impression physique ou la conformité PDF/A/PDF/UA. Pas de calque d’arrière-plan, mosaïque, animation, vérification de signature ou DRM.
