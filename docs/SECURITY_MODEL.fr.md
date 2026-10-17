# Assainissement et caviardage des documents

Le traitement reste local. L'assainissement des structures et le caviardage visible sont deux opérations
distinctes ; le [modèle anglais](SECURITY_MODEL.md) donne les limites techniques exactes.

**Assainir une nouvelle copie** / `sunipdf-cli sanitize` retire métadonnées, signets, destinations nommées,
pièces jointes, JavaScript et actions automatiques/additionnelles. Annotations de pièces jointes et
multimédias retirées ; contenu visible, annotations ordinaires et valeurs des formulaires conservés.
Les liens utilisant les destinations supprimées peuvent cesser de fonctionner.

Les ressources de polices identifiées comme inutilisées sont détachées et les objets inaccessibles
supprimés avant réécriture complète. Un nom réutilisé ambigu reste conservé ; tous les sous-ensembles
inutilisés ne sont donc pas nécessairement retirés. Polices Type3/ExtGState, syntaxes/filtres non pris en
charge et structures malformées sont refusés. Résultat rouvert et structures nettoyées vérifiées avant
publication. Chiffrement, mots de passe et permissions conservés ; aucun fichier existant remplacé,
même avec `--force`. Un échec laisse le document actif et les fichiers d'origine inchangés.

Limites : entrée 128 Mio, 250 000 objets, un million de nœuds directs, profondeur 32, contenu décodé
8 Mio par flux/page et 256 Mio au total. L'original, les anciennes copies, sauvegardes, instantanés de
récupération et l'historique Annuler gardent leurs données. Aucun effacement sécurisé du disque ou des
sauvegardes externes. Texte/images visibles, OCR, formulaires et données applicatives personnalisées
hors des structures nettoyées ne sont pas anonymisés. Contrôler la copie avant partage. Une réécriture
invalide les signatures numériques ; garder l'original signé si nécessaire.

Appliquer un caviardage retire le contenu traversant les zones, réécrit le PDF et vérifie l'absence de
texte extractible dessous. Le repli prudent actuel peut convertir un objet ou une page en image et
faire perdre la sélection du texte conservé. Rectangle noir, annotation, recadrage ou marques non
appliquées ne suppriment pas le contenu sous-jacent.

Aucune certification PDF/A/PDF/UA ou garantie contre toutes les variantes hostiles. Contrôles structurels,
rendu natif, lecteurs indépendants/corpus, matériel, validation archivistique des signatures et fuzzing
exhaustif restent des preuves distinctes.
