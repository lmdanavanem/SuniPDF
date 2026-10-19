# SuniPDF 0.32.0

## Nouveautés et corrections

- Réorganisation d’un objet ou de toute la sélection par glisser-déposer dans le panneau Objets, trait d’insertion, refus des déplacements obsolètes et annulation par Échap.
- Commande JSON `reorder-objects` : destination finale explicite, ordre relatif, sauvegarde et mots de passe existants.
- Contrôle de permission de modification avant changement de l’ordre ; les destinations invalides et opérations identiques sont refusées/ignorées avant mutation.
- La version publique 0.31.0 rejoint les régressions de mise à jour portable et MSI.

[Périmètre et utilisation](OBJECT_ORDER.fr.md).

Paquets Windows non signés ; aucune clé indépendante de publication disponible. ARM64 est compilé sans exécution matérielle. Les vérifications natives, imprimantes, lecteurs tiers et certifications PDF/A/PDF/UA restent séparées.

## Validation

Qualification en cours ; les résultats observés seront consignés dans l’état du projet.
