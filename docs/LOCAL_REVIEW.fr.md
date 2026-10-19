# Comparaison locale et recherche dans plusieurs PDF

[English](LOCAL_REVIEW.md)

Implémenté pour la 0.31.0. La qualification exécutée figure dans [État du projet](PROJECT_STATUS.fr.md).

## Rechercher dans les fichiers sélectionnés

Choisissez **Édition → Rechercher dans des fichiers PDF…**, sélectionnez explicitement les fichiers,
saisissez le texte littéral et lancez l’analyse. La sensibilité à la casse est optionnelle. Sans cette option,
la recherche utilise la conversion Unicode en minuscules, sans analyse linguistique, suppression des accents
ni expressions régulières. Chaque page correspondante présente son nombre d’occurrences, un extrait borné
conservant la casse originale et un bouton ouvrant sa page physique dans un onglet.

L’analyse porte sur les fichiers enregistrés : enregistrez d’abord vos modifications. Les résultats concernent
les octets lus pendant cette analyse ; ouvrir un résultat ouvre le fichier actuellement enregistré.
Relancez l’analyse après toute modification. Aucun parcours de dossier, index persistant, OCR automatique,
télémétrie ni envoi réseau. L’ordre du texte reste celui de PDFium ; les colonnes non balisées, images et
données cachées ne reçoivent aucune interprétation sémantique. Utilisez d’abord l’OCR explicite pour les scans.

Les fichiers terminés sans correspondance restent distincts des fichiers annulés, limités, absents, invalides,
protégés par mot de passe ou interdisant la copie. Un échec n’empêche pas les fichiers suivants d’être analysés.
L’annulation conserve les pages déjà analysées et signale les fichiers incomplets. Un nombre de pages inconnu
reste inconnu, au lieu de devenir zéro. Les résultats restent en mémoire jusqu’à remplacement ou fermeture.

## Comparer deux versions

Choisissez **Fichier → Comparer des fichiers PDF…**, sélectionnez l’original et la révision enregistrés,
puis leurs premières pages physiques. Les paires sont alignées successivement à partir de ces positions.
Ajustez-les après une insertion : pas d’alignement automatique, détection du texte déplacé ni différence sémantique.

Le résumé compare le texte extrait, les dimensions et la présence des pages ainsi que leurs aperçus rendus.
Cliquez sur une paire modifiée pour afficher l’original, la révision et les pixels différents en rouge.
Une même échelle est utilisée, avec un grand côté de 768 pixels maximum ; les pages sont alignées en haut
à gauche et les écarts RGB inférieurs ou égaux à 16/255 sont ignorés. Les zones absentes sont blanches.
Une différence de présence ou de dimensions reste signalée même pour un aperçu vide.

Une absence de différence observée **ne prouve pas l’identité des PDF**. Les changements petits ou pâles
peuvent rester sous la résolution ou la tolérance. Ni graphes d’objets, métadonnées, pièces jointes, scripts,
données cachées, signatures, accessibilité ni conformité archivistique ne sont comparés. Aucun OCR.
Le JSON signale les paires sans texte extrait des deux côtés. Les deux fichiers doivent autoriser la copie
selon la politique existante. Le bureau ne recueille pas de mots de passe pour ces opérations ; les échecs
restent explicites. La CLI accepte un même secret pour tous les fichiers via les sources environnement/stdin.

La comparaison enregistre le SHA256 des octets exacts. Un aperçu relit les fichiers, vérifie les deux empreintes
et refuse une source modifiée avant rendu. Il ne présente jamais silencieusement une nouvelle révision comme
un ancien résultat. Ces empreintes identifient les octets analysés ; elles ne signent pas les publications.

## Limites et annulation

- Recherche : 64 fichiers explicitement sélectionnés maximum, chacun limité à 64 Mio ; comparaison de deux fichiers de même limite.
- 250 000 caractères PDFium par page maximum avant extraction complète.
- Recherche : 10 000 pages analysées et 2 000 pages correspondantes maximum par analyse.
- Comparaison : 1 à 500 paires, 200 par défaut. Toute paire restante rend le rapport incomplet.
- Un thread moteur traite successivement fichiers et pages. Seule la paire d’aperçus demandée est conservée en textures.
- Annulation entre opérations natives : un appel PDFium actif n’est pas interrompu de force.
  Ces bornes ne constituent pas un plafond de mémoire native ou de temps CPU face à des PDF hostiles.

## Interface en ligne de commande

```powershell
sunipdf-cli search-files original.pdf revision.pdf --query "contrat" --match-case
sunipdf-cli compare original.pdf revision.pdf --left-start 2 --right-start 3 --max-pages 200
sunipdf-cli search-files protege.pdf --query "contrat" --password-env SUNIPDF_PASSWORD
```

Les commandes produisent du JSON sans modifier les sources. Les pages physiques commencent à 1.
Un même mot de passe s’applique à tous les fichiers sélectionnés et n’apparaît jamais dans le rapport.
Le code 0 indique une analyse bornée terminée, y compris sans correspondance ou avec des différences.
Le code 2 accompagne le rapport JSON complet lorsque l’analyse reste partielle. Les erreurs fatales de
comparaison, ouverture ou arguments conservent les conventions d’erreur JSON et de codes de sortie existantes.
Consultez `complete`, les états/erreurs et les compteurs : un fichier non analysé ne doit pas être considéré comme vérifié.

## Qualification

Les régressions couvrent texte/pixels identiques ou différents, pages ajoutées, décalages manuels, dimensions,
pages vides, fichiers/texte excessifs, entrées absentes/invalides, AES-128/256, permissions de copie,
annulation, préservation des octets sources, refus des aperçus obsolètes, codes de sortie partiels et secrets hors argv.
Les tests du bureau couvrent les commandes, l’annulation à la fermeture et la reprise après erreur.
Cela ne qualifie pas Narrator, du matériel ARM64 ni une interopérabilité universelle avec les lecteurs tiers.
