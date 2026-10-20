# Préparer une signature visuelle

Une signature visuelle est une image d’écriture. Elle ne prouve ni une identité ni l’intégrité du document. La signature par certificat reste une fonction distincte.

## Dessiner

Dans **Remplir et signer**, choisir une signature ou un paraphe puis dessiner. **Lisser les traits** est facultatif et désactivé initialement. L’option permet de comparer les mêmes points bruts avec un aperçu courbe. Annuler un trait et Effacer agissent toujours sur le dessin original. L’aperçu et le placement emploient la même géométrie, y compris les points isolés et les extrémités.

Un brouillon accepte au plus 20 000 points échantillonnés et 256 traits. Si une limite est atteinte, le placement est désactivé et un message l’explique. Annuler un trait ou effacer le brouillon permet de reprendre ; un dessin tronqué n’est pas accepté silencieusement comme signature terminée.

## Importer une photo ou un scan

Choisir une image dans la boîte de signature. Activer **Retirer le fond**, puis éventuellement **Réduire les ombres du papier** pour de l’encre sombre et fine sur papier clair avec un éclairage irrégulier. Régler le seuil et vérifier l’aperçu avant placement. L’affichage réduit l’aperçu, tandis que le placement utilise les mêmes pixels préparés en pleine résolution. Toute modification d’un réglage invalide l’aperçu précédent.

L’image décodée originale reste disponible pour modifier les réglages. Le fichier source n’est pas édité. Les limites de décodage, la correction d’orientation et la taille de travail de 1600 pixels restent appliquées. Le nettoyage peut supprimer des marques grises pâles ou affecter les traits larges et les fonds texturés ; le désactiver si l’aperçu ne convient pas. Il ne s’agit pas d’un détourage photo général.

## Ligne de commande

```powershell
sunipdf-cli prepare-signature-image photo.jpg --background adaptive --threshold 200 --out signature.png
sunipdf-cli prepare-signature-image scan.png --background threshold --out signature.png
sunipdf-cli prepare-signature-image handwriting.png --background keep --out signature.png
```

`keep` est le mode par défaut. `threshold` applique le seuil global de luminance ; `adaptive` compense d’abord la luminosité locale du papier. Le seuil accepte 60 à 250. La sortie doit être un PNG, les marges transparentes sont rognées avec six pixels de marge et un résultat vide est refusé. La commande fonctionne sans ouvrir de PDF ni initialiser le moteur de rendu. Son JSON décrit la sortie et indique explicitement `cryptographic_signature: false`.

L’entrée doit être un fichier ordinaire d’au plus 64 Mio lors du contrôle de taille ; les limites de dimensions et d’allocation du décodeur s’appliquent aussi. Une sortie existante exige `--overwrite`. Remplacer le chemin d’entrée est refusé même avec cette option. Les erreurs suivent les conventions JSON et codes de sortie existantes. Le PNG demandé est un fichier utilisateur ordinaire non chiffré ; la bibliothèque facultative de signatures protégée par DPAPI utilise un autre chemin de stockage.

## Qualification

Les tests du moteur et du bureau couvrent l’encre et l’alpha, les images vides ou trop grandes, la conservation des points bruts, les limites de dessin et la liaison entre réglages et aperçu. La CLI x64 empaquetée est aussi contrôlée sur des images générées avec Pillow réservé aux tests et fixé par empreintes : pixels conservés, nettoyage des ombres, huit orientations JPEG EXIF, gardes d’entrée/sortie, placement et rendu PDF, contrôle structurel indépendant qpdf. Ce lecteur d’images n’est pas livré comme dépendance de l’application.

Ces contrôles ne qualifient pas l’interaction native souris/tablette, Narrateur, les photos réelles de clients ni le matériel ARM64. Les preuves de release indiquent les contrôles effectivement exécutés.
