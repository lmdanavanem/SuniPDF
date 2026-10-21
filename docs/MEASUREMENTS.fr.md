# Mesures locales calibrées (0.40.0)

[English](MEASUREMENTS.md) · [Backlog bâtiment](AEC_BACKLOG.fr.md)

Ouvrez **Commentaire → Annotations enrichies**, puis **Mesures calibrées**. Choisissez longueur, polyligne, périmètre, surface polygonale ou angle. Saisissez un sommet `x,y` par ligne, en points de page affichée, origine en bas à gauche après rotation et rognage. Pour un angle, le sommet est le point du milieu. Les polygones se ferment automatiquement : ne répétez pas le premier point.

Saisissez deux points délimitant une distance connue sur le plan, cette distance positive et l'unité `mm`, `cm`, `m`, `in` ou `ft`. Pieds et pouces sont décimaux : 10 pieds 6 pouces correspondent à `10.5 ft` ou `126 in`. Choisissez 0 à 4 décimales. Le résultat du brouillon se recalcule à la saisie. Cliquez sur **Aperçu**, vérifiez le rendu réel, puis **Appliquer**. Toute modification impose un nouvel aperçu. Annuler/Échap n'applique rien. Enregistrez le PDF ; un Annuler retire l'annotation et ses données ensemble.

Chaque annotation conserve sa propre calibration, géométrie, unité et précision. Une mesure ne change pas silencieusement l'échelle d'une autre. Ce premier lot utilise la saisie de coordonnées ; sélection directe de points, accrochage, échelle de page, vues et recalcul global restent au backlog. Recréez la mesure pour changer géométrie ou échelle. Déplacement, redimensionnement, style et modification générique du texte sont refusés afin d'éviter des valeurs périmées. Suppression et réponses aux commentaires restent disponibles selon les droits du PDF.

## CLI et données

```powershell
sunipdf-cli measure input.pdf --kind length --points "40,70 140,70" --calibration "40,40 140,40" --known-distance 10 --unit m --precision 2 --page 1 --out measured.pdf
sunipdf-cli measure input.pdf --kind area --points "40,70 140,70 140,120 40,120" --calibration "40,40 140,40" --known-distance 10 --unit m --out measured-area.pdf
sunipdf-cli measurements measured-area.pdf
```

Les exemples donnent 10 m et 50 m². La création renvoie l'indice d'annotation et le résultat. `measurements` exporte par page les données JSON : identité de l’annotation, sommets, calibration, valeur numérique non arrondie, libellé et précision. Surface en unités carrées (`m2`), angle en degrés (`deg`), autres résultats dans l'unité linéaire choisie. Aucune formule de tableur, macro ou référence externe. Pages à partir de 1, indices d'annotation à partir de 0. Pour un PDF protégé, préférez `--password-env NOM` ou `--password-stdin`.

`measure` écrit toujours une nouvelle copie ; original et destination existante sont refusés même avec `--overwrite`. Le droit d'annotation suffit. AES-128/AES-256 et les deux mots de passe sont conservés ; le droit de modification du contenu n'est pas nécessaire.

## Format ouvert et limites

Une annotation Stamp possède une apparence PDF et une chaîne JSON `SuniPDFMeasurement`. La version 1 stocke `coordinate_system: "pdf-user-space"`, type, sommets et deux points de calibration en coordonnées utilisateur PDF, distance connue, unité, précision et rectangle. Le calcul physique reste invariant sous rotation/rognage. L'export recalcule depuis ces entrées, sans utiliser un commentaire modifiable comme résultat. Versions inconnues, métadonnées invalides, rectangles déplacés/redimensionnés par un autre éditeur et métadonnées importées `/Measure` reconnues sont explicitement refusés.

Ce format ne prétend pas être compatible avec les métadonnées propriétaires Bluebeam. Une mesure modifiée ailleurs peut devoir être recréée. L'annotation n'est ni une signature numérique ni une preuve de justesse dimensionnelle : vérifiez distance connue et révision du plan.

Export JSON limité à 1000 pages et 4096 mesures, avec 4 Mio de métadonnées par page. Maximum 128 sommets, coordonnées finies dans la page, calibration positive, aucun segment nul, polygones simples non dégénérés et résultat entre 1e-9 et 1e12. Croisements et géométries invisibles sont refusés. Le libellé peut couvrir le plan : vérifiez l'aperçu.

Les cas synthétiques de dimensions connues vérifient calculs, conversions métriques/impériales, rotations/rognage, réouverture, aperçu/application, Annuler, protection, refus et métadonnées périmées. La CLI ajoute MuPDF et qpdf indépendants. Cela ne qualifie ni précision topographique, ni plans clients, impression physique, interaction native/Narrateur, PDF/A ou PDF/UA.

Échelle de page et recalcul, manipulation directe, vues multi-échelles, comptages, volumes, registre et reconnaissance restent ouverts dans AEC-001–026. Le local reste gratuit et open source ; cloud et collaboration sont reportés.
