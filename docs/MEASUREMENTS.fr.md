# Mesures locales calibrées

[English](MEASUREMENTS.md) · [Backlog bâtiment](AEC_BACKLOG.fr.md)

**0.44.0 en préparation :** modification des mesures, retour à l'échelle enregistrée et corrections sont implémentés et en cours de validation. Les nouvelles procédures ci-dessous décrivent cette version de développement, pas un paquet publié. [Périmètre et audit préalable](RELEASE_0.44.0.fr.md).

**Disponible en [0.43.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.43.0):** échelles de page persistantes, inspection et réutilisation explicite, en complément du tracé direct. Les mesures existantes conservent leurs valeurs. [Preuves](qa/RELEASE_0.43.0.json).

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

En 0.44, **Utiliser l'échelle enregistrée** abandonne la calibration temporaire de la page lorsqu'un défaut valide existe. Sinon, **Effacer l'échelle temporaire** retire uniquement cette calibration. Ces contrôles ne modifient ni le PDF ni les autres pages. Une échelle enregistrée indisponible le reste ; calibrez explicitement la page avant de tracer une mesure étalonnée.

## Inspecter les mesures enregistrées

Dans **Mesures**, ouvrez **Mesures enregistrées** pour la page courante. Consulter la liste ne modifie pas le PDF. Elle présente valeur et unité, type, indice d'annotation, identifiant éventuel, nombre de sommets, précision et distance connue des mesures non angulaires. Les listes sont paginées par 50 ; **Actualiser** relit l'état courant. Les métadonnées incompatibles ou modifiées sont signalées explicitement. En 0.44, données et libellés sont partagés en mémoire au lieu de recopier toute la liste et de régénérer le JSON à chaque image.

**Voir sur la page** sélectionne l'annotation réelle et ouvre ses commentaires. **Utiliser cette échelle** copie la calibration d'une mesure non angulaire dans la session de la page, sans modifier l'annotation ni enregistrer d'échelle commune. Les références hors de la page affichée sont refusées. Cliquez séparément sur **Enregistrer l’échelle dans le PDF** pour la retrouver aux prochaines sessions. Fermez l'inspecteur avant de tracer ou de changer de document.

Déplacement, redimensionnement, style et modification générique du texte sont refusés afin d'éviter des valeurs périmées. Suppression et réponses aux commentaires restent disponibles selon les droits du PDF. La saisie manuelle reste disponible dans **Annotations enrichies → Mesures calibrées** : les coordonnées `x,y` sont des points de page affichée, origine en bas à gauche après rotation et rognage. En 0.44, ce chemin ne fournit plus de distance positive arbitraire : renseignez explicitement une calibration valide.

## Modifier une mesure enregistrée — 0.44 en préparation

1. Ouvrez **Mesures enregistrées** sur la page concernée et cliquez sur **Modifier la mesure** près de la valeur.
2. Modifiez distance de référence connue, unité ou précision. Passer de `m` à `cm` convertit une référence de 10 m en 1000 cm sans changer sa taille physique. Changer la distance recalcule uniquement cette mesure. Pour les angles, seule la précision est modifiable.
3. Demandez un **Aperçu**, vérifiez le candidat rendu, puis cliquez sur **Appliquer**. Sommets et points de référence restent fixes dans cet éditeur. Identité, auteur, réponses et champs du tableau des annotations sont conservés ; échelles de page et autres mesures restent inchangées.
4. **Annuler** conserve le PDF intact. Une modification réussie correspond à une étape Annuler/Rétablir. Enregistrez le PDF pour la conserver sur disque.

Un changement de paramètre invalide l'aperçu précédent. Document modifié, enregistrement périmé, identité verrouillée ou ambiguë, métadonnées/apparences incompatibles et texte de valeur ou géométrie modifiés ailleurs provoquent un refus. Actualisez ou rouvrez l'inspecteur avant de réessayer ; conservez les annotations incompatibles plutôt que de les remplacer par supposition. L'édition exige le droit d'annotation et concerne les mesures compatibles de documents jusqu'à 128 Mio. Les sommets à la souris restent à développer ; la CLI permet leur remplacement explicite.

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

Pour la 0.44, exportez un enregistrement attendu récent avant de modifier. L'indice d'annotation vient de cet enregistrement, pas de sa position dans la liste :

```powershell
$inventory = sunipdf-cli measurements measured.pdf --page 1 | ConvertFrom-Json
$record = $inventory.pages[0].measurements[0]
$record | ConvertTo-Json -Depth 16 | Set-Content -Encoding utf8 expected-measurement.json
sunipdf-cli edit-measurement measured.pdf --page 1 --index $record.index --expected-record expected-measurement.json --unit cm --precision 1 --out edited.pdf
```

Cet exemple convertit les 10 m d'origine en 1000 cm sans changer la taille physique. `--expected-record` accepte un enregistrement complet de version 1 avec identité stable, limité à 64 Kio. Page, indice, identité, géométrie, calibration et précision courants doivent correspondre. Réexportez après modification ; les enregistrements périmés sont refusés. `--points` remplace jusqu'à 128 sommets de page affichée ; `--kind` change le type si la géométrie convient. Pour remplacer la référence, fournissez `--calibration` avec `--known-distance` dans l'unité finale, ou choisissez explicitement `--use-page-calibration`. Passer d'un angle à une mesure étalonnée exige une référence physique. `measure --kind angle` demande trois points et aucune calibration.

En 0.44, `measurements` et `page-calibration` exigent le droit de copie. Une création/modification permise par le droit d'annotation peut réussir sans ce droit, mais sa réponse JSON contient `measurement: null` et `export_unavailable: true` ; elle n'exporte ni calibration enregistrée ni quantité. `edit-measurement` exige aussi une nouvelle destination, même avec `--overwrite`.

Les exemples manuels donnent 10 m et 50 m² ; celui avec échelle enregistrée donne 1000 cm. `--use-page-calibration` exclut `--calibration` et `--known-distance` ; l'absence d'échelle valide provoque un refus. `--unit` convertit physiquement la distance de référence. `page-calibration` renvoie un objet `calibration` ou `null` par page ; sans `--page`, la lecture est limitée à 1000 pages. Enregistrement et retrait exigent une page explicite et ne modifient jamais les valeurs existantes.

La création renvoie l'indice d'annotation et le résultat. `measurements` exporte par page les données JSON : identité de l’annotation, sommets, calibration, valeur numérique non arrondie, libellé et précision. Surface en unités carrées (`m2`), angle en degrés (`deg`), autres résultats dans l'unité linéaire choisie. Aucune formule de tableur, macro ou référence externe. Pages à partir de 1, indices d'annotation à partir de 0. Pour un PDF protégé, préférez `--password-env NOM` ou `--password-stdin`.

`measure`, `set-page-calibration` et `remove-page-calibration` écrivent toujours de nouvelles copies ; original et destinations existantes sont refusés même avec `--overwrite`. Changer l'échelle de page exige le droit de modification du document ; créer une mesure demande seulement le droit d'annotation. AES-128/AES-256 et les deux mots de passe sont conservés.

## Format ouvert et limites

Une annotation Stamp possède une apparence PDF et une chaîne JSON `SuniPDFMeasurement`. La version 1 stocke `coordinate_system: "pdf-user-space"`, type, sommets et deux points de calibration en coordonnées utilisateur PDF, distance connue, unité, précision et rectangle. Le calcul physique reste invariant sous rotation/rognage. L'export recalcule depuis ces entrées, sans utiliser un commentaire modifiable comme résultat. Versions inconnues, métadonnées invalides, rectangles déplacés/redimensionnés par un autre éditeur et métadonnées importées `/Measure` reconnues sont explicitement refusés.

Une chaîne de page distincte `SuniPDFPageCalibration` conserve l'échelle par défaut versionnée pour les nouvelles mesures. Limitée à 4096 octets, elle contient les points utilisateur PDF et les gardes de géométrie/contenu. La vérification accepte au maximum 1024 flux de page, 8 Mio décodés par flux et 32 Mio au total. Elle ne remplace pas la calibration propre à chaque annotation.

Ce format ne prétend pas être compatible avec les métadonnées propriétaires externes. Une mesure modifiée ailleurs peut devoir être recréée. L'annotation n'est ni une signature numérique ni une preuve de justesse dimensionnelle : vérifiez distance connue et révision du plan.

Export JSON limité à 1000 pages et 4096 mesures, avec 4 Mio de métadonnées par page. Maximum 128 sommets, coordonnées finies dans la page, calibration positive, aucun segment nul, polygones simples non dégénérés et résultat entre 1e-9 et 1e12. Croisements et géométries invisibles sont refusés. Le libellé peut couvrir le plan : vérifiez l'aperçu.

Les cas synthétiques de dimensions connues vérifient calculs, conversions métriques/impériales, rotations/rognage, réouverture, aperçu/application, Annuler, protection, refus et métadonnées périmées. Les ajouts 0.43 couvrent persistance, permissions, conversion physique et contrôles qpdf dans le corpus existant ; les résultats exécutés figurent dans l'état du projet. Cela ne qualifie ni précision topographique, ni plans clients, impression physique, interaction native/Narrateur, PDF/A ou PDF/UA.

Les changements 0.44 sont en validation. Le redimensionnement du contenu avec annotations refuse atomiquement les pages mesurées au lieu de laisser rectangles et quantités incohérents. Sommets à la souris, recalcul par lot, accrochage aux vecteurs PDF, vues multi-échelles, volumes et reconnaissance avancée restent ouverts. AEC-001/002/003/004/026 restent partiels ; échelles de page, inspecteur et modification individuelle ne terminent pas leurs critères généraux. Les [comptages manuels et le tableau des annotations](MARKUP_REGISTER.fr.md) existent déjà dans leurs limites documentées en 0.41.0. Le local reste gratuit et open source. La [roadmap](ROADMAP.fr.md) distingue bibliothèque d'annotations prévue en 0.45, colonnes/totaux/export de métrés en 0.46 et superpositions/révisions en 0.47.
