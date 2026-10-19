# SuniPDF 0.30.1

[English](RELEASE_0.30.1.md)

Ce correctif de sécurité renforce le nettoyage des documents, le changement de protection, les signatures
numériques et la CLI. La publication et les validations exécutées figurent dans l'[état du projet](PROJECT_STATUS.fr.md).

- L'assainissement retire les paquets XFA et annotations multimédias Screen puis vérifie leur absence.
  Le nettoyage par défaut du caviardage retire aussi XFA et les textes de remplacement d'accessibilité
  dans les contenus/ressources conservés. Un contenu ambigu impossible à réécrire sûrement est refusé.
- Remplacer la protection d'un document chiffré demande le mot de passe propriétaire actuel. La
  protection existante reste en place jusqu'à la réussite de l'opération.
- L'intégrité des signatures PDF accepte les CMS détachés avec SHA-256, SHA-384 ou SHA-512 ; empreintes
  faibles/non prises en charge et contenu CMS attaché sont refusés. La confiance d'identité exige aussi
  une intégrité valide et des preuves Windows de chaîne/révocation utilisables.
- La signature et sa vérification portent sur les octets chargés ou enregistrés. Un fichier modifié,
  manquant ou non enregistré demande un rechargement ou un enregistrement préalable. Les déclarations
  du signataire sont limitées, filtrées et séparées du verdict. Sujet/émetteur du certificat sont plus
  complets ; les PFX contenant plusieurs identités privées ambiguës sont refusés.
- La CLI lit les mots de passe PDF depuis une variable d'environnement ou l'entrée UTF-8 redirigée.
  Les anciens arguments restent disponibles avec avertissement. La sortie texte filtre les contrôles
  du terminal ; le JSON échappe les contrôles C1 sans changer les valeurs décodées. `signatures` imprime
  son rapport complet puis retourne 2 en cas d'échec. La date reste une déclaration du signataire.
- Les gardes Office refusent données binaires intégrées et expressions conditionnelles dans les ODF
  sans macros pris en charge. Les versions publiées ignorent les variables de développement pour les
  bibliothèques natives, l'OCR, le stockage des signatures et le sélecteur de fichiers. Au démarrage,
  l'application Windows restreint les répertoires de recherche des DLL.
- Les actions des workflows sont figées par commit vérifié. L'archive WiX officielle de secours est
  contrôlée par SHA256 avant extraction ; ZIP/MSI publics 0.30.0 rejoignent les régressions de mise à jour.

Le metteur à jour dépend encore de GitHub HTTPS et des sommes de contrôle distribuées par GitHub.
Ces contrôles détectent la corruption mais n'authentifient pas indépendamment l'éditeur en cas de
compromission du compte de distribution. Aucune clé de publication n'est disponible : la signature
indépendante des métadonnées reste à réaliser. Les paquets Windows restent également non signés.
Matériel ARM64, imprimantes physiques et tests hostiles exhaustifs restent à qualifier. Le nettoyage
ne certifie pas la conformité PDF/A ou PDF/UA.

Le rapport privé fourni n'est pas distribué. Comportements et limites :
[nettoyage](SECURITY_MODEL.fr.md), [signatures](SIGNATURE_SECURITY.fr.md),
[caviardage](REDACTION.fr.md) et [conversion Office](OFFICE_CONVERSION.fr.md).

## Validation et distribution

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.1) sous `lmdanavanem`, release `417149160`, source immuable `697a963c756b316e1eb1e678298886bc08521da1`.

Le [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39235567730) et la [CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39234732417) réussissent sur cette source : **575 tests**, zéro échec, 9 ignorés par défaut ; **1060 clés EN/FR**, **84 contrôles qpdf du corpus** et **9 sorties de fonctions indépendantes**, **4 cas de polices**, rendu logiciel DX12, **14 allers-retours Office**, 4 PDF sous licence/744 pages, **22 scénarios portables**, **16 références MSI/80 journaux de transactions**. CodeQL Rust, Python et Actions réussissent sans alerte ouverte.

Les six fichiers publics et les copies `dist/` correspondent aux paquets inspectés et aux SHA256. Huit régressions de sécurité passent aussi avec la CLI publiée. Le code du metteur à jour a vérifié le ZIP x64 de **27,029,229 octets** en **1.21 s**, sans installation sur ce poste, avec le harnais déjà compilé et le module inchangé.

[Preuves détaillées](qa/RELEASE_0.30.1.json).
