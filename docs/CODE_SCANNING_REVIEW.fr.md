# Revue Code Scanning — 2026-10-17

## Complément 0.51.0 — 23 octobre 2026

Les alertes #195 et #196 ont été revues individuellement sur la [source `527c91f`](https://github.com/lmdanavanem/SuniPDF/blob/527c91f1e82a6832f4e1ae55610a4caf67dc12c9/crates/sunipdf-core/tests/certificates.rs#L228-L239), puis classées `used in tests` par `lmdanavanem`. Ces entrées synthétiques appartiennent uniquement à un test d’intégration non-Windows : le fournisseur indisponible refuse l’identité avant tout import ou toute signature. Aucun secret de production ni identifiant par défaut identifié. Les tests et les requêtes restent actifs, sans exclusion. Le [relevé des décisions](qa/CODE_SCANNING_0.51.0.json) conserve les horodatages et commentaires exacts de classement.

## Complément 0.31.0

Quatre nouvelles alertes #178–181 concernent uniquement des mots de passe de PDF synthétiques dans des cibles de tests Cargo. Revue individuelle sous `lmdanavanem`, classées « used in tests » ; aucun secret de production confirmé. Aucune exclusion ; CodeQL Rust/Python/Actions réussit sur la source publiée sans alerte ouverte. [CSV](qa/CODE_SCANNING_0.31.0.csv) · [QA](qa/RELEASE_0.31.0.json).

## Complément 0.30.1 - 19 octobre 2026

22 nouvelles alertes revues individuellement sous `lmdanavanem` : 21 limitées aux tests et un faux positif sur le candidat de mot de passe propriétaire vide. Aucun secret de production confirmé. CodeQL Rust/Python/Actions réussit sur la source publiée, sans alerte ouverte ni exclusion. [CSV des décisions](qa/CODE_SCANNING_0.30.1.csv) · [Preuves de release](qa/RELEASE_0.30.1.json).

[English](CODE_SCANNING_REVIEW.md) · [Inventaire critique par alerte (CSV)](qa/CODE_SCANNING_2026-10-16.csv)

## Revue Code Scanning 0.30.0 — 2026-10-17

La [première analyse complète](https://github.com/lmdanavanem/SuniPDF/actions/runs/38994022178) de la source
`b87beea529196e7f93308908eef1e96be4feff25` a terminé ses jobs Rust, Python et Actions avec succès et relevé
**27 nouvelles critiques de fixtures de test** (#129–155) et **cinq alertes élevées de pointeurs natifs** (#124–128).

Les 27 critiques ont été vérifiées individuellement sur cette source exacte, puis écartées par
`lmdanavanem` avec le motif `used in tests`. Il s’agit de mots de passe déterministes de PDF chiffrés et
d’identités PFX synthétiques dans des tests d’intégration Cargo ou des modules `cfg(test)` explicites.
Aucun identifiant de production ni valeur d’authentification par défaut de l’application n’a été identifié.
Le [CSV par alerte](qa/CODE_SCANNING_0.30.0_2026-10-17.csv) conserve périmètre, source, emplacement,
motif, auteur, horodatage et commentaire de clôture vérifié pour chaque constat.

Les cinq alertes élevées ont été corrigées par des gardes explicites et une propriété contrôlée des
pointeurs de certificats, chaînes et horodatages natifs, avec des régressions de certificat nul ou incomplet.
La [nouvelle analyse](https://github.com/lmdanavanem/SuniPDF/actions/runs/38995917746) a réussi ses trois jobs
sur la source `4896cc277c32621e5d65335b4fd9823760427793`. GitHub déclare #124–128 **corrigées** au
`2026-10-17T02:51:43Z` ; elles n’ont pas été écartées. L’API authentifiée non filtrée a ensuite confirmé
**zéro alerte ouverte**. Requêtes, fixtures et couverture restent actives, sans exclusions.

L’[analyse finale Rust, Python et Actions](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673553)
a réussi sur la source immuable de release `4db31c0ee62cfae9d65239edf1eac93db147841e`.
L’API authentifiée a confirmé zéro alerte ouverte au `2026-10-17T04:01:22.4366426Z`, puis le processus de
publication l’a revérifié avant de rendre la [0.30.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.0)
Latest stable. La [CI main](https://github.com/lmdanavanem/SuniPDF/actions/runs/38999673471) et le
[workflow complet du tag](https://github.com/lmdanavanem/SuniPDF/actions/runs/39000390231) ont réussi sur
cette source, dont 551 tests, 21 scénarios de mise à jour portable et 15 bases MSI / 75 journaux de transaction.
Les [preuves de release](qa/RELEASE_0.30.0.json) consignent source exacte, empreintes des paquets,
téléchargements publics, vérification du metteur à jour et relecture Projects. Ce résultat établit les
contrôles consignés, pas une absence exhaustive de vulnérabilités ni la qualification du matériel non testé.

## Corrections 0.21.0 vérifiées

[CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648825) a réussi ses trois jobs Rust/Actions/Python
sur la source `f006b7dd027fd5a3084c2bfffc2fceaa1b66be46`. L'API authentifiée non filtrée confirme **zéro alerte ouverte** au `2026-10-16T19:59:54.8976555Z`.
**#3 élevée corrigée à `2026-10-16T19:58:04Z` ; #119 moyenne corrigée à `2026-10-16T19:49:12Z`.**
Elles n'ont pas été écartées : contrôle natif du certificat explicite et permissions root `contents: read` vérifiés par une nouvelle analyse.
#119 est la réindexation du constat de release #1 ; #1 et #2 restent des corrections historiques distinctes.

Les **115 critiques** ont été écartées individuellement par **`lmdanavanem`** le 2026-10-16 entre 19:34:00 et
19:38:10 UTC : **114 `used in tests`**, **#13 `false positive`** (essai d'authentification vide, aucun secret incorporé).
Le [CSV](qa/CODE_SCANNING_2026-10-16.csv) conserve chaque champ historique et ajoute état, motif, auteur,
horodatage et commentaire vérifiés. Les fixtures et requêtes sont conservées ; aucune exclusion de dossier.
L'analyse Rust contient encore **115 constats déjà triés**, Actions/Python zéro ; ce résultat ne signifie pas zéro vulnérabilité dans le projet.

471 tests locaux et CI, formatage/Clippy, 912 clés, 75 qpdf, corpus 4/744, autotest, 19 scénarios portables,
13 bases MSI/65 journaux, ARM64, six paquets/sidecars publics et metteur à jour réussis. Trois nouvelles régressions
CMS et DTD/entités XMP/XFDF UTF-8/UTF-16 réussies ; parsing XML 1.0 borné et notices conservés.
[0.21.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.0) était Latest stable à sa publication. Preuves dans l'[état du projet](PROJECT_STATUS.fr.md).
Fuzzing exhaustif, confiance/révocation/horodatage des certificats et matériel restent non vérifiés.

L'instantané initial ci-dessous est historique : ses états ouverts décrivent la revue initiale, et non l'état actuel.

## Instantané initial examiné

Les **117 alertes ouvertes** de l'instantané API GitHub authentifié ont toutes été analysées : **115 critiques,
une élevée et une moyenne**. L'instantané comporte aussi **une alerte moyenne déjà corrigée (#2)** et **zéro
alerte écartée**. Aucune autre alerte critique fermée n'y figure. Ce sont les états GitHub ; une classification
issue de la lecture du code ne ferme pas une alerte.

L'analyse automatique vise le commit `72dd623576203988a22fdc9662963b7982c7c746` ; la source 0.20.0 examinée est
`54054b101282a7688dde778b24090af6d6e1e6a6`. Les fichiers applicatifs et workflows portant les alertes sont
identiques entre ces commits ; la différence suivie est `.github/FUNDING.yml`. Les emplacements ci-dessous
renvoient à cette source.

## Résultats

| Gravité GitHub | Alertes | Résultat de la revue |
|---|---:|---|
| Critique | 114 | Entrées de test déterministes ; aucun identifiant de production identifié dans ces alertes |
| Critique | 1 (#13) | Tentative intentionnelle d'authentification PDF par mot de passe vide ; aucun secret incorporé |
| Élevée | 1 (#3) | Faux positif pour le chemin signalé partant d'un pointeur nul ; confiance élevée selon le code et les contrats API Windows |
| Moyenne | 1 (#1) | Permissions du workflow non déclarées explicitement ; lacune réelle, actuellement compensée par les droits par défaut en lecture seule |
| **Total ouvert** | **117** | **117 analysées ; aucune restante dans cet instantané** |
| Moyenne, déjà corrigée | 1 (#2) | Déclaration des permissions du workflow CI ; état historique corrigé, séparé du total ouvert |

Aucun secret de production n'a été confirmé parmi les 115 alertes critiques. Ce résultat ne démontre pas
l'absence de vulnérabilités dans tout le dépôt. Les 117 alertes restent ouvertes dans l'instantané examiné ;
cette revue n'a écarté aucune alerte, changé ni configuration de scanning ni permissions, ni modifié le code
applicatif ou les workflows.

## Critiques : valeurs cryptographiques codées en dur

Les **115/115** relèvent de `rust/hard-coded-cryptographic-value`, sur **105 emplacements distincts**,
**23 fichiers** et **39 fonctions englobantes**. Les doublons GitHub au même emplacement gardent des lignes
CSV distinctes. Le CSV conserve liens des alertes, fonctions exactes, preuves de périmètre test,
classifications, emplacements d'appui et extraits expurgés ; aucune valeur d'identifiant n'y est copiée.

Les **114 alertes limitées aux tests** comprennent **97 alertes de tests d'intégration, 16 dans des modules
explicitement `#[cfg(test)]` et une dans un utilitaire de test**. Chaque emplacement et fonction englobante
a été inspecté ; attributs de test, modules conditionnels et appels des utilitaires établissent ce périmètre.
Mots de passe fixes et entrées invalides/vides/Unicode vérifient protection PDF et échecs. Les tests de
signature génèrent des identités RSA-2048 éphémères dans des dossiers temporaires ; leur phrase secrète PFX
fixe n'est ni une clé privée livrée ni un identifiant de compte. Aucune alerte examinée n'a été classée comme
constante algorithmique nécessaire.

### Alerte de production #13

[`security::load`](https://github.com/lmdanavanem/SuniPDF/blob/54054b101282a7688dde778b24090af6d6e1e6a6/crates/sunipdf-core/src/security.rs#L162-L199)
essaie le mot de passe utilisateur PDF vide si l'appelant n'en fournit pas. La chaîne vide n'est pas un secret
stocké. `load_mem_with_options` authentifie toujours ; des identifiants invalides deviennent `PasswordRequired`
et un résultat encore chiffré est refusé. Cela permet la protection par mot de passe propriétaire seul sans
donner le droit de supprimer les restrictions. `unprotect` vérifie séparément le mot de passe propriétaire.
Le writer prend les mots de passe fournis par l'appelant et obtient le matériau de clé AES-256 avec `getrandom`.
Les régressions existantes couvrent protection propriétaire seule et mauvais mot de passe.
[L'alerte #13](https://github.com/lmdanavanem/SuniPDF/security/code-scanning/13) reste ouverte.

Conserver les entrées de régression déterministes. Les randomiser ou masquer les littéraux uniquement pour
faire taire l'analyseur ne corrigerait pas une vulnérabilité et pourrait affaiblir les tests. Un triage ultérieur
doit conserver la justification de chaque alerte ; ce résumé ne justifie pas l'exclusion de dossiers entiers.

## Élevée #3 : accès à un pointeur invalide signalé

[`rust/access-invalid-pointer`](https://github.com/lmdanavanem/SuniPDF/security/code-scanning/3) vise
`certificate_native.rs:222`, avec une trace partant de `null_mut` ligne 185. Le chemin signalé partant du
pointeur nul est un **faux positif avec confiance élevée** : la construction paresseuse
`(!cert.is_null()).then(|| Cert(cert))` ne crée le propriétaire du certificat que pour un pointeur non nul ;
`ok_or(...)?` retourne avant `cert_info` si le pointeur est nul. Une erreur API sort également avant cet appel.
Il s'agit d'une conclusion sur le code et les contrats API, pas d'un résultat de fuzzing CMS malformé.

Windows retourne un contexte de certificat signataire possédé via le paramètre de sortie de
[`CryptVerifyDetachedMessageSignature`](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-cryptverifydetachedmessagesignature).
L'`Option<Cert>` reste vivante pendant que `cert_info` emprunte son pointeur ; `Cert::drop` libère le contexte
ensuite ou lors d'un retour en erreur. Le seul autre appelant, `Identity::info`, utilise un certificat dont le
résultat d'énumération a été contrôlé non nul avant construction, selon le contrat de
[`CertEnumCertificatesInStore`](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-certenumcertificatesinstore).
Windows décode le [`CERT_CONTEXT`](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/ns-wincrypt-cert_context) ;
aucun appelant ne convertit de mémoire PDF arbitraire vers cette structure. Aucun déréférencement nul
atteignable ni libération prématurée n'a été trouvé par ces deux appelants.

Les tests natifs existants couvrent signature/vérification, altération, PFX invalide et mots de passe erronés/vides.
Ils n'épuisent pas les entrées CMS malformées et ne forcent pas une API réussie avec un certificat nul.
Clarification facultative : une branche explicite de contrôle du pointeur pourrait exprimer le même invariant
plus directement sans ajouter de wrapper.

## Moyenne #1 : permissions du workflow de release

[`actions/missing-workflow-permissions`](https://github.com/lmdanavanem/SuniPDF/security/code-scanning/1)
est une **lacune confirmée de déclaration explicite du moindre privilège** : `release.yml` ne déclare pas
`permissions` à la racine ni sur le job de build. Le `contents: write` du job Publish séparé ne limite pas le build.

La lecture authentifiée des paramètres du dépôt a retourné `default_workflow_permissions: "read"` et
`can_approve_pull_request_reviews: false`. Le build hérite donc de droits en lecture seule ; aucun token
excessivement doté en écriture n'a été observé. Le workflow tourne sur tags/déclenchement manuel, pas sur
événements de pull requests non fiables. Élargir les droits par défaut plus tard, ou copier le workflow ailleurs,
rendrait l'omission plus importante. Cette alerte seule ne prouve pas une compromission.

Durcissement futur minimal : déclarer `permissions: { contents: read }` à la racine et conserver le
`contents: write` explicite de Publish, selon les [règles GitHub de permissions workflow/job](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#permissions).
Cette revue consigne le constat sans changer ces paramètres ni fichiers.

## Historique : alerte #2 déjà corrigée

[L'alerte #2](https://github.com/lmdanavanem/SuniPDF/security/code-scanning/2), autre constat moyen
`actions/missing-workflow-permissions` dans `ci.yml`, est déjà **corrigée** dans l'instantané API
(`fixed_at: 2026-10-16T18:51:19Z`). CI déclare désormais `contents: read` à la racine. Cette correction
historique est distincte de l'alerte #1 du workflow de release et n'a pas été réalisée par cette revue.

## Limites

La couverture statique des alertes ouvertes de cet instantané est complète ; ce n'est ni un test d'intrusion
de tout le projet ni une garantie de sécurité. La revue n'a exécuté aucun nouveau test Cargo, d'exploitation
ou de fuzzing, et n'a changé aucun état d'alerte GitHub. Les régressions de release existantes sont documentées
séparément dans l'[état du projet](PROJECT_STATUS.fr.md).
