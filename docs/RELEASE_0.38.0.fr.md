# SuniPDF 0.38.0

## Changements

- Six zones d’en-tête/pied modifiables dans Pages → Numéros de page et la commande `headers-footers`.
- Numéro absolu et total des pages, taille, marges, sélection et coordonnées tournées/recadrées.
- Refus atomique du dépassement, conservation du chiffrement autorisé et nouvelle copie en CLI.
- Régression indépendante étendue à 42 scénarios, 148 appels CLI et deux contrôles qpdf.

[Utilisation et limites](PAGE_DECORATIONS.fr.md).

## Validation

La qualification Windows, CodeQL, les paquets et la publication sur la source finale restent à exécuter. Les régressions locales du moteur et de la CLI passent ; l’interaction native reste à qualifier.

## Limites

Zones sur une ligne, colonnes égales et texte de premier plan. Le contenu existant peut chevaucher : inspecter avant d’enregistrer. Pas de date automatique, règles paires/impaires, qualification des écritures complexes ou certification de conformité. Paquets non signés et ARM64 uniquement compilé en croisé. Les autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Projects reste différé à la demande du propriétaire.
