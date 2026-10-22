# Mesures locales calibrées

[English](MEASUREMENTS.md) · [Backlog bâtiment](AEC_BACKLOG.fr.md)

**0.43.0 en préparation :** les échelles de page enregistrées et l'inspecteur décrits ci-dessous complètent le bouton direct et le tracé publiés en [0.42.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.42.0). Publication et qualification figurent dans [l'état du projet](PROJECT_STATUS.fr.md).

## Mesurer directement sur la page

1. Ouvrez un PDF et cliquez sur **Mesures**, à côté de **Tous les outils** dans la barre principale. L'outil est aussi disponible dans **Tous les outils → Mesures** et le premier groupe de l'espace **Commenter**. Son libellé reste visible dans les fenêtres étroites.
2. Choisissez `mm`, `cm`, `m`, `in` ou `ft` et 0 à 4 décimales. Cliquez sur **Calibrer cette page**, puis sur deux points distincts délimitant une distance connue du plan. Saisissez cette **Distance connue**, strictement positive, puis cliquez sur **Confirmer l’échelle**. Pieds et pouces sont décimaux : 10 pieds 6 pouces correspondent à `10.5 ft` ou `126 in`.
3. Choisissez le type de mesure et cliquez directement sur la page. **Longueur** ouvre l'aperçu après deux points. **Angle** l'ouvre après trois points : premier côté, sommet, second côté ; un angle ne nécessite pas d'échelle physique. Pour **Polyligne**, **Périmètre** et **Surface polygonale**, placez les sommets puis cliquez sur **Prévisualiser la mesure**, appuyez sur **Entrée** ou double-cliquez sur le dernier point. Les polygones se ferment automatiquement : ne répétez pas le premier sommet.
4. Vérifiez le rendu réel et la valeur, puis cliquez sur **Appliquer**. Rien n'est ajouté avant la réussite de cette application. **Annuler** ou **Échap** abandonne l'aperçu. Enregistrez le PDF pour conserver la mesure ; un Annuler retire l'annotation et ses données ensemble. L'outil reste actif après l'application : vous pouvez tracer la mesure suivante sur la page calibrée.

Pendant le tracé, **Retour arrière** ou **Annuler le point** retire le dernier point. **Maj** aligne le segment suivant horizontalement ou verticalement dans les axes de la page affichée ; il ne s'agit pas d'un accrochage aux traits du PDF. **Échap** ou **Annuler le tracé** abandonne le brouillon. Terminez ou annulez le tracé avant de mesurer une autre page. Un changement du document annule également les brouillons.

## Échelle de session et échelle de page enregistrée

Chaque page peut avoir une échelle confirmée distincte pendant la session. Les modifications ordinaires d'annotations et les changements habituels d'onglet conservent ces échelles ; les changements de géométrie, de structure ou de contenu des pages les effacent. L'échelle de session est prioritaire sur l'échelle par défaut enregistrée pour cette page. La navigation n'attribue jamais l'échelle d'une page à une autre. Un changement d'unité convertit la distance confirmée au lieu de réinterpréter sa valeur numérique.

Après confirmation, cliquez sur **Enregistrer l’échelle dans le PDF**, puis enregistrez le fichier PDF pour la conserver sur disque. L'opération réussie remplace la calibration de session par l'échelle de page enregistrée. À la réouverture, une échelle valide est chargée automatiquement pour les nouvelles mesures. Une échelle uniquement conservée en session disparaît à la fermeture du document.

**Retirer l’échelle enregistrée** supprime seulement l'échelle par défaut de cette page ; une calibration de session choisie explicitement reste disponible. Enregistrez le PDF pour conserver le retrait. **Annuler** et **Rétablir** restaurent l'état correspondant du document et relisent son échelle ; ils ne restaurent pas une échelle de session effacée. L'ajout ou le retrait d'une échelle de page exige le droit de modification du document ; tracer une annotation exige le droit d'annotation. Les contrôles d'enregistrement/retrait restent désactivés sans le droit de modification du document. La protection AES-128/AES-256 est conservée.

L'échelle enregistrée contient des points utilisateur PDF, les limites de page et une empreinte du contenu décodé. Rotation et réorganisation des pages conservent une échelle compatible. Rognage modifié, redimensionnement, contenu modifié, métadonnées incompatibles et `UserUnit` non standard sont refusés plutôt que traités comme des échelles valides. Recalibrez ou retirez une échelle périmée reconnue ; les données inconnues ou malformées sont préservées et signalées. Ces contrôles n'authentifient pas toutes les révisions possibles : vérifiez toujours le plan et sa dimension connue.

Chaque annotation appliquée conserve ses propres calibration, géométrie, unité et précision. Enregistrer, retirer ou remplacer l'échelle de page ne recalcule jamais les annotations existantes. Le recalcul global et les vues multi-échelles nommées restent ouverts.

## Inspecter les mesures enregistrées

Dans **Mesures**, ouvrez **Mesures enregistrées** pour la page courante. La liste en lecture seule présente valeur et unité, type, indice d'annotation, identifiant éventuel, nombre de sommets, précision et distance connue des mesures non angulaires. Les listes sont paginées par 50 ; **Actualiser** relit l'état courant. Les métadonnées incompatibles ou modifiées sont signalées explicitement.

**Voir sur la page** sélectionne l'annotation réelle et ouvre ses commentaires. **Utiliser cette échelle** copie la calibration d'une mesure non angulaire dans la session de la page, sans modifier l'annotation ni enregistrer d'échelle commune. Les références hors de la page affichée sont refusées. Cliquez séparément sur **Enregistrer l’échelle dans le PDF** pour la retrouver aux prochaines sessions. Fermez l'inspecteur avant de tracer ou de changer de document.

Recréez une mesure appliquée pour changer son échelle ou ses sommets. Déplacement, redimensionnement, style et modification générique du texte sont refusés afin d'éviter des valeurs périmées. Suppression et réponses aux commentaires restent disponibles selon les droits du PDF. La saisie manuelle reste disponible dans **Annotations enrichies → Mesures calibrées** : les coordonnées `x,y` sont des points de page affichée, origine en bas à gauche après rotation et rognage.

## CLI et données

```powershell
sunipdf-cli measure input.pdf --kind length --points "40,70 140,70" --calibration "40,40 140,40" --known-distance 10 --unit m --precision 2 --page 1 --out measured.pdf
sunipdf-cli measure input.pdf --kind area --points "40,70 140,70 140,120 40,120" --calibration "40,40 140,40" --known-distance 10 --unit m --out measured-area.pdf
sunipdf-cli measurements measured-area.pdf

sunipdf-cli set-page-calibration input.pdf --page 1 --calibration "40,40 140,40" --known-distance 10 --unit m --out calibrated.pdf
sunipdf-cli page-calibration calibrated.pdf --page 1
sunipdf-cli measure calibrated.pdf --kind length --points "40,70 140,70" --use-page-calibration --unit cm --out measured-cm.pdf
sunipdf-cli remove-page-calibration measured-cm.pdf --page 1 --out without-page-scale.pdf
```

Les exemples manuels donnent 10 m et 50 m² ; celui avec échelle enregistrée donne 1000 cm. `--use-page-calibration` exclut `--calibration` et `--known-distance` ; l'absence d'échelle valide provoque un refus. `--unit` convertit physiquement la distance de référence. `page-calibration` renvoie un objet `calibration` ou `null` par page ; sans `--page`, la lecture est limitée à 1000 pages. Enregistrement et retrait exigent une page explicite et ne modifient jamais les valeurs existantes.

La création renvoie l'indice d'annotation et le résultat. `measurements` exporte par page les données JSON : identité de l’annotation, sommets, calibration, valeur numérique non arrondie, libellé et précision. Surface en unités carrées (`m2`), angle en degrés (`deg`), autres résultats dans l'unité linéaire choisie. Aucune formule de tableur, macro ou référence externe. Pages à partir de 1, indices d'annotation à partir de 0. Pour un PDF protégé, préférez `--password-env NOM` ou `--password-stdin`.

`measure`, `set-page-calibration` et `remove-page-calibration` écrivent toujours de nouvelles copies ; original et destinations existantes sont refusés même avec `--overwrite`. Changer l'échelle de page exige le droit de modification du document ; créer une mesure demande seulement le droit d'annotation. AES-128/AES-256 et les deux mots de passe sont conservés.

## Format ouvert et limites

Une annotation Stamp possède une apparence PDF et une chaîne JSON `SuniPDFMeasurement`. La version 1 stocke `coordinate_system: "pdf-user-space"`, type, sommets et deux points de calibration en coordonnées utilisateur PDF, distance connue, unité, précision et rectangle. Le calcul physique reste invariant sous rotation/rognage. L'export recalcule depuis ces entrées, sans utiliser un commentaire modifiable comme résultat. Versions inconnues, métadonnées invalides, rectangles déplacés/redimensionnés par un autre éditeur et métadonnées importées `/Measure` reconnues sont explicitement refusés.

Une chaîne de page distincte `SuniPDFPageCalibration` conserve l'échelle par défaut versionnée pour les nouvelles mesures. Limitée à 4096 octets, elle contient les points utilisateur PDF et les gardes de géométrie/contenu. La vérification accepte au maximum 1024 flux de page, 8 Mio décodés par flux et 32 Mio au total. Elle ne remplace pas la calibration propre à chaque annotation.

Ce format ne prétend pas être compatible avec les métadonnées propriétaires externes. Une mesure modifiée ailleurs peut devoir être recréée. L'annotation n'est ni une signature numérique ni une preuve de justesse dimensionnelle : vérifiez distance connue et révision du plan.

Export JSON limité à 1000 pages et 4096 mesures, avec 4 Mio de métadonnées par page. Maximum 128 sommets, coordonnées finies dans la page, calibration positive, aucun segment nul, polygones simples non dégénérés et résultat entre 1e-9 et 1e12. Croisements et géométries invisibles sont refusés. Le libellé peut couvrir le plan : vérifiez l'aperçu.

Les cas synthétiques de dimensions connues vérifient calculs, conversions métriques/impériales, rotations/rognage, réouverture, aperçu/application, Annuler, protection, refus et métadonnées périmées. Les ajouts 0.43 couvrent persistance, permissions, conversion physique et contrôles qpdf dans le corpus existant ; les résultats exécutés figurent dans l'état du projet. Cela ne qualifie ni précision topographique, ni plans clients, impression physique, interaction native/Narrateur, PDF/A ou PDF/UA.

Modification des sommets enregistrés, recalcul explicite des annotations existantes, accrochage aux vecteurs PDF, vues multi-échelles, volumes et reconnaissance avancée restent ouverts dans AEC-001–026. L'échelle de page et l'inspecteur de la 0.43 ne clôturent pas ces cartes. Les [comptages manuels et le tableau des annotations](MARKUP_REGISTER.fr.md) existent déjà dans leurs limites documentées en 0.41.0. Le local reste gratuit et open source.
