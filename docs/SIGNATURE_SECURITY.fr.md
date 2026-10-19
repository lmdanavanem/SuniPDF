# Décisions sur le stockage et la vérification des signatures

Implémentation jusqu'à la version 0.30.1. Les résultats exécutés sont consignés dans
`PROJECT_STATUS.fr.md` ; ce document décrit les garanties et limites.

Depuis 0.30.1, l'intégrité exige un CMS détaché avec SHA-256, SHA-384 ou SHA-512. MD5, SHA-1,
autres empreintes et contenu CMS attaché sont refusés. `identity_trusted` exige simultanément
l'intégrité actuelle valide, le certificat du signataire et la réussite des contrôles chaîne/révocation.
La méthode `SignatureTrust::chain_and_revocation_trusted()` porte uniquement sur chaîne/révocation.

L'application vérifie les octets précisément chargés/enregistrés. Fichier disque modifié ou manquant
et modifications non enregistrées imposent rechargement/enregistrement ; un changement ultérieur ne
substitue pas les octets contrôlés. Champ, motif, lieu et date déclarés sont filtrés, limités à 256
caractères et séparés du verdict. Sujet/émetteur utilisent des noms X.500 bornés. Les PFX sont limités
à 16 Mio/256 certificats ; plusieurs identités avec clé privée sont refusées.

La CLI `signatures` imprime son JSON complet puis retourne 2 si une vérification échoue. Un document
sans signature garde le code 0 et les deux indicateurs de vérification à false. `failure_cause` fournit
la raison exploitable par programme ; `declared_signing_time`/`signing_time_source` identifient une date
déclarative. L'ancien champ `signing_time` reste un alias de cette déclaration.

## Signatures visuelles réutilisables

L’enregistrement reste facultatif. Décocher **Enregistrer sur cet ordinateur pour la réutiliser**
permet la pose sans créer de fichier dans la bibliothèque. Ces images ne sont pas des signatures
numériques : elles ne prouvent pas l’identité et restent copiables depuis un PDF.

Les nouveaux fichiers de `%APPDATA%\SuniPDF\signatures` portent l’extension `.dpapi`. Windows DPAPI
chiffre le PNG pour l’utilisateur actuel, avec une entropie propre à l’application et sans clé commune
à la machine. Une DACL protégée autorise seulement le SID de cet utilisateur. L’en-tête versionné
contient le SID public, sans clé de récupération. Le PNG est décodé en mémoire ; aucun fichier temporaire
déchiffré ni envoi en ligne n’est créé.

Les anciens fichiers `signature-<chiffres>.png` et `initials-<chiffres>.png` sont migrés individuellement
à leur lecture. L’application valide le PNG borné, chiffre dans un fichier temporaire voisin à DACL privée,
synchronise et publie sans écraser une destination existante, puis déchiffre le fichier persistant et compare
les octets exacts. Elle ne supprime le PNG qu’après avoir aussi vérifié que l’original n’a pas changé.
Un fichier chiffré existant différent, inaccessible ou corrompu provoque un refus : les deux originaux restent.
En cas d’échec, l’ancien PNG reste utilisable avec un avertissement ; il n’est jamais présenté comme protégé.

La bibliothèque refuse les traversées de répertoires parents, points de réanalyse, noms inconnus, fichiers
trop volumineux et PNG de plus de 1600 pixels par côté ou 32 Mio d’allocation de décodage. Les PNG encodés
sont limités à 16 Mio, plus une marge bornée pour l’en-tête et DPAPI. Les anciennes images externes dépassant
ces limites restent sur disque et peuvent être importées séparément avec l’import habituel.

Conservez l’image d’origine ou une sauvegarde permettant de récupérer le profil Windows et ses clés DPAPI.
Copier seulement les `.dpapi` vers un autre utilisateur, perdre ou réinitialiser le profil ne permet pas de
les récupérer. L’application conserve les fichiers illisibles et demande le profil d’origine ou une nouvelle
importation ; elle ne réinitialise ni ne remplace les clés. La corbeille demande toujours deux clics et supprime
l’entrée sélectionnée. Il s’agit d’une suppression ordinaire, sans effacement garanti des anciens secteurs ou
sauvegardes. Une personne utilisant votre session déjà ouverte peut réutiliser les images.

## Vérification des certificats

L’intégrité CMS du PDF et la confiance dans son signataire sont deux résultats distincts. Un CMS détaché
valide relie la signature aux plages d’octets sauvegardées sans authentifier à lui seul une identité. Les
révisions ultérieures, plages invalides et CMS non pris en charge restent signalés séparément.

Le contrôle par défaut utilise le magasin Windows actuel, l’usage de signature de documents, les algorithmes
forts et la politique de certificats. La révocation est indiquée séparément : bonne, révoquée, hors ligne,
inconnue ou non vérifiée. Les deux récupérations d’URL, certificats et révocations, restent limitées au cache ;
AIA et la mise à jour automatique des racines sont désactivés. Une racine approuvée sans preuve de révocation
utilisable ne produit pas `identity_trusted: true`. Le magasin Windows fait partie de cette décision, sans
équivalence avec AATL/EUTL ou une signature qualifiée.

Chaque clic explicite sur **Vérifier** nécessite de cocher l’autorisation réseau pour cette vérification.
Elle est réinitialisée après l’action et à la réouverture. Windows peut contacter les serveurs de certificats,
AIA, CRL ou OCSP et transmettre des identifiants de certificats ; le PDF n’est pas envoyé. La révocation a un
délai cumulatif de cinq secondes ; les mises à jour automatiques des racines restent désactivées. Échec et preuves
périmées restent indisponibles ou inconnus, sans résultat positif inventé. La CLI vérifie hors ligne par défaut.

Les horodatages RFC3161 du CMS et l’OID RFC3161 Windows sont pris en charge. CryptoAPI vérifie le jeton contre
la valeur de signature CMS d’origine. Sont contrôlés : algorithme fort, EKU exclusif critique d’horodatage,
usage de clé de signature, plausibilité temporelle, validité du certificat TSA à la date du jeton, chaîne TSA et
révocation. La valeur PDF `/M` reste déclarative. Les anciennes contresignatures sont explicitement non prises
en charge. Un jeton mathématiquement valide issu d’une TSA non approuvée reste non approuvé ; sans révocation
vérifiable, aucun horodatage de confiance n’est annoncé. Aucune demande à une TSA ni création d’horodatage
n’est ajoutée à la signature.

Les conteneurs `/DSS` de certificats, OCSP et CRL sont inspectés avec des bornes de nombre, taille et type.
Leur présence est indiquée **incorporée, non vérifiée**, sans prétendre authentifier une LTV d’archivage.
La politique historique des listes de confiance, l’authentification d’archives OCSP/CRL et le renouvellement
d’horodatages documentaires restent à réaliser. AATL/EUTL, jetons matériels PKCS#11, signatures qualifiées et
certification auprès de lecteurs tiers nécessitent des travaux séparés.

Primitives Windows : [DPAPI](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata),
[chaînes de certificats](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-certgetcertificatechain),
[vérification d’horodatage](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-cryptverifytimestampsignature).
