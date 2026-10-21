# Mesures locales calibrées

[English](MEASUREMENTS.md) · [Backlog bâtiment](AEC_BACKLOG.fr.md)

**Intégration non publiée :** l'accès direct et le tracé sur la page décrits ci-dessous sont préparés après la 0.41.0. Ils ne figurent pas dans les paquets 0.41.0 publiés. Les données de mesures PDF et la CLI existantes restent prises en charge.

## Mesurer directement sur la page

1. Ouvrez un PDF et cliquez sur **Mesures**, à côté de **Tous les outils** dans la barre principale. L'outil est aussi disponible dans **Tous les outils → Mesures** et le premier groupe de l'espace **Commentaire**. Son libellé reste visible dans les fenêtres étroites.
2. Choisissez `mm`, `cm`, `m`, `in` ou `ft` et 0 à 4 décimales. Cliquez sur **Calibrer cette page**, puis sur deux points distincts délimitant une distance connue du plan. Saisissez cette **Distance connue**, strictement positive, puis cliquez sur **Confirmer l’échelle**. Pieds et pouces sont décimaux : 10 pieds 6 pouces correspondent à `10.5 ft` ou `126 in`.
3. Choisissez le type de mesure et cliquez directement sur la page. **Longueur** ouvre l'aperçu après deux points. **Angle** l'ouvre après trois points : premier côté, sommet, second côté ; un angle ne nécessite pas d'échelle physique. Pour **Polyligne**, **Périmètre** et **Surface polygonale**, placez les sommets puis cliquez sur **Prévisualiser la mesure**, appuyez sur **Entrée** ou double-cliquez sur le dernier point. Les polygones se ferment automatiquement : ne répétez pas le premier sommet.
4. Vérifiez le rendu réel et la valeur, puis cliquez sur **Appliquer**. Rien n'est ajouté avant la réussite de cette application. **Annuler** ou **Échap** abandonne l'aperçu. Enregistrez le PDF pour conserver la mesure ; un Annuler retire l'annotation et ses données ensemble. L'outil reste actif après l'application : vous pouvez tracer la mesure suivante sur la page calibrée.

Pendant le tracé, **Retour arrière** ou **Annuler le point** retire le dernier point. **Maj** aligne le segment suivant horizontalement ou verticalement dans les axes de la page affichée ; il ne s'agit pas d'un accrochage aux traits du PDF. **Échap** ou **Annuler le tracé** abandonne le brouillon. Terminez ou annulez le tracé avant de mesurer une autre page. Un changement du document annule également les brouillons.

## Durée de l'échelle et mesures enregistrées

Chaque page peut avoir une échelle confirmée distincte pendant la session du document. Les modifications ordinaires d'annotations et les changements habituels d'onglet conservent ces échelles ; les changements de géométrie ou de structure des pages les effacent. Fermer puis rouvrir un document impose une nouvelle calibration. La navigation entre pages n'attribue pas l'échelle d'une page à une autre. Un changement d'unité convertit la distance confirmée au lieu de réinterpréter sa valeur numérique.

L'échelle de session n'est pas enregistrée comme métadonnée commune à la page. Chaque annotation appliquée conserve ses propres calibration, géométrie, unité et précision, y compris après enregistrement et réouverture. Recalibrer les prochaines mesures ne modifie pas les annotations existantes. La sauvegarde d'une échelle commune, le recalcul global et les vues multi-échelles restent ouverts.

Recréez une mesure appliquée pour changer son échelle ou ses sommets. Déplacement, redimensionnement, style et modification générique du texte sont refusés afin d'éviter des valeurs périmées. Suppression et réponses aux commentaires restent disponibles selon les droits du PDF. La saisie manuelle reste disponible dans **Annotations enrichies → Mesures calibrées** : les coordonnées `x,y` sont des points de page affichée, origine en bas à gauche après rotation et rognage.

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

Modification des sommets enregistrés, échelles de page communes persistantes et recalcul, accrochage aux vecteurs PDF, vues multi-échelles, volumes et reconnaissance avancée restent ouverts dans AEC-001–026. Les [comptages manuels et le tableau des annotations](MARKUP_REGISTER.fr.md) existent déjà dans leurs limites documentées en 0.41.0. Le local reste gratuit et open source ; cloud et collaboration sont reportés.
