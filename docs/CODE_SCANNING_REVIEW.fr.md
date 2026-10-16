# Revue Code Scanning — 2026-10-16

[English](CODE_SCANNING_REVIEW.md) · [Inventaire critique par alerte (CSV)](qa/CODE_SCANNING_2026-10-16.csv)

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
