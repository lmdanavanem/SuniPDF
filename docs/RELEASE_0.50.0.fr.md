# SuniPDF 0.50.0 — commandes de lecture et outils organisés

[English](RELEASE_0.50.0.md) · [Guide de lecture et des outils](READER_WORKSPACE.fr.md)

Cette version comprend le périmètre ci-dessous. Les fichiers publiés et la qualification sont suivis sur la [page de version 0.50.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.50.0).

## Changements

- Retrouvez 18 outils dans cinq familles consultables et filtrables depuis **Outils**. Mesures, bibliothèque d’annotations, table des quantités et superposition de révisions partagent **Réviser et mesurer**.
- Basculez le volet gauche entre Outils, Pages et Signets. La barre principale conserve une entrée Outils générale, sans raccourcis distincts pour les mesures et la bibliothèque.
- Utilisez quatre sections dans le panneau de mesure droit : Créer une mesure, Échelle de la page, Tracé et Données enregistrées. L’espace Mesures n’ajoute plus de barre horizontale au-dessus du PDF.
- Affichez un seul panneau contextuel à droite. Un brouillon de mesure actif bloque l’ouverture des autres panneaux ; quitter explicitement l’outil ou en changer annule le brouillon local et conserve les échelles confirmées.
- Naviguez entre les pages et réglez le zoom sous le PDF. Les valeurs telles que **149%** restent sur une ligne ; ajustements à la page/largeur et menu compact Options de lecture restent disponibles lorsque la fenêtre rétrécit.
- Distinguez rotation de lecture et rotation des pages PDF. Le rail droit ouvre les panneaux ; la zone d’état affiche les messages d’opération.
- Conservez l’édition des sommets enregistrés et ses régressions sur paquets publics. Les paquets publiés 0.49.0 rejoignent les régressions de mise à jour portable et MSI.

## Qualification

Les preuves et fichiers de distribution sont suivis sur la [page de version 0.50.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.50.0). Les résultats concernent la source exacte qui y figure.

## Périmètre

Cette version réorganise les parcours existants de lecture et d’outils ; elle n’introduit ni nouvelle commande CLI, ni nouveau type de mesure. Permissions, gardes d’aperçu/application et limites des mesures enregistrées restent applicables. AEC-002 et AEC-003 restent partiels.

Les preuves UI automatisées hors écran concernent uniquement les dispositions et interactions testées. Elles ne qualifient ni fenêtres natives, périphériques physiques, lecteurs d’écran ou ARM64 natif. Les paquets restent non signés ; les critères généraux 1.0 restent ouverts.
