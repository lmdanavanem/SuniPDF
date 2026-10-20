# SuniPDF 0.36.0

## Changements

- Personnalisation des raccourcis dans Aide et la palette, avec filtre, refus des conflits, désactivation et restauration.
- Préférences atomiques ; Annuler abandonne le brouillon et un échec d’écriture conserve les réglages actifs. Les installations existantes gardent les valeurs par défaut.
- Combinaisons actives dans menus, accueil, infobulles et palette ; protection des variantes avec Shift et de la saisie.
- Paquets 0.35.0 ajoutés aux contrôles de mise à niveau ZIP/MSI conservés. Aucune dépendance applicative ajoutée.

[Utilisation et limites](KEYBOARD_SHORTCUTS.fr.md).

## Validation

CI finale, CodeQL, qualification des paquets et publication à exécuter. Tests egui sans fenêtre des entrées, préférences et annulation ; aucune validation native clavier, Narrateur ou matériel revendiquée.

## Limites

Seules les commandes de la table existante sont personnalisables ; touches système, navigation, presse-papiers et édition de texte restent réservées. Explorer reste à faire. Paquets non signés et sans clé de publication indépendante ; ARM64 compilé sans exécution matérielle. Les autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Modifications Projects différées à la demande du propriétaire.
