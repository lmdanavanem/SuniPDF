# Conversion Office locale : portée et limites

SuniPDF crée un fichier séparé et refuse toute sortie déjà existante. L’original reste inchangé. L’export utilise le document PDF ouvert, y compris ses modifications en mémoire. Les marques de caviardage non appliquées et une édition de texte en cours bloquent l’export depuis l’interface. Les restrictions de copie sont respectées ; les formats Office ne conservent ni le chiffrement ni les signatures numériques PDF.

## Office vers PDF

Sous Windows, un **LibreOffice 26.2 ou 26.8 déjà installé** est nécessaire. SuniPDF recherche `soffice.com` dans les dossiers Program Files habituels et vérifie sa version avant conversion. En son absence, l’interface indique clairement que la conversion est indisponible. L’application ne télécharge ni n’installe de convertisseur. Chaque document utilise un profil temporaire isolé ; les sessions et profils Office existants ne sont ni modifiés ni fermés.

À partir de la 0.39.0, la vérification de version utilise les mêmes options sans interface que la conversion et un budget annulable de 30 secondes au démarrage à froid. La conversion conserve sa limite de 120 secondes ; les gardes de chemins, archives, versions, diagnostics et sorties restent actives.

Les entrées acceptées sont DOCX, XLSX, PPTX, ODT, ODS et ODP sans macros. Les anciens formats binaires, macros, formules, champs actifs, objets incorporés, liens externes, DTD/entités XML, chemins d’archive dangereux et images déguisées sont refusés. Les images doivent être des PNG ou JPEG. Cette politique conservatrice peut aussi refuser des documents valides complexes.

Limites : 1 à 128 documents par lot, 64 Mio par entrée, 4 096 éléments d’archive, 256 Mio après décompression, 16 Mio par partie XML et 64 Mio de XML au total. Chaque conversion dispose de 120 secondes ; le PDF produit est limité à 256 Mio. Le traitement est séquentiel et annulable. Une erreur sur un document est signalée séparément. Le résultat est rouvert et contrôlé avant écriture atomique ; les octets source doivent toujours correspondre.

Les profils temporaires désactivent les macros, contenus actifs, scripts Python et automatisation OLE. Les références externes sont refusées avant lancement et un proxy local inutilisable est configuré. Ces protections applicatives **ne constituent pas un bac à sable réseau du système d’exploitation**. Les documents ne sont pas téléversés. Les polices absentes et la mise en page peuvent produire des différences : vérifiez les PDF obtenus.

| Profil mémorisé | Politique d’images |
| --- | --- |
| Standard | JPEG qualité 90, réduction à 150 ppp |
| Impression | Sans perte, sans réduction |

Aucun profil ne revendique la conformité PDF/A ou PDF/UA. Seul le nom du profil est mémorisé dans les réglages, jamais le contenu des documents.

## PDF vers Office

| Format | Résultat et limites |
| --- | --- |
| Word | Texte visible simple, non pivoté, sur une colonne, remis en paragraphes de 11 pt. Polices, espacements, pagination, graphiques et tableaux sémantiques ne sont pas reconstruits. Formulaires, masques, effets, calques optionnels, texte bidirectionnel et remplissage blanc ou inconnu sont refusés. |
| Excel | Une feuille par page ; les objets texte alignés deviennent des cellules de chaînes littérales. Aucun calcul de formule, typage numérique, fusion de cellules, style original ou reconstruction sémantique de tableaux. Les colonnes ambiguës et pages complexes sont refusées. |
| PowerPoint | Une image PNG de 150 ppp par page, ajustée sans rognage. Texte, tableaux et vecteurs ne sont pas modifiables. Les pages de tailles différentes reçoivent des marges. Maximum : 8 millions de pixels et 8 192 pixels par côté. |

Tous les exports sont limités à 1–128 pages source et à un paquet de 128 Mio. Word et Excel ont des plafonds supplémentaires sur le texte, les cellules, les objets et les flux PDF. Un refus, une erreur ou une annulation ne modifie ni l’original ni une sortie existante. `source_pages` désigne les pages PDF traitées, sans garantir la pagination du document Word.

## Validation et mesures

Les tests synthétiques contrôlent le XML, les valeurs de cellules ressemblant à des formules, les pixels PNG, les refus, l’annulation et la conservation des sources. Ils ne suffisent pas à démontrer le rendu d’une application Office particulière.

Le test ignoré `office_fidelity_corpus` mesure les pages échantillons des quatre publications NIST/NBS épinglées par empreinte. Les résultats complexes acceptés ou refusés restent séparés d’un document synthétique simple clairement identifié. Lorsqu’un convertisseur est exigé en CI, DOCX, XLSX et PPTX doivent chacun effectuer une conversion réelle avec les deux profils. Le rapport relève les textes, dimensions, empreintes et erreurs de pixels réellement observés ; aucun pourcentage de fidélité n’est inventé. Les téléchargements et rapports restent hors des paquets de l’application.

Les résultats d’exécution doivent être enregistrés dans les preuves de publication avant clôture du lot. Aucun LibreOffice n’était installé sur le poste de développement lors de l’implémentation ; le contrôle réel utilise un exécuteur GitHub jetable et un installateur officiel épinglé.

Références et détails techniques : [notice anglaise](OFFICE_CONVERSION.md), [corpus et licences](../scripts/regression-corpus.json), [installateur réservé à la CI](../scripts/test-libreoffice.json).
