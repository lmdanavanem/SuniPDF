# SuniPDF 0.50.0 — commandes de lecture et outils organisés

[English](RELEASE_0.50.0.md) · [Guide de lecture et des outils](READER_WORKSPACE.fr.md)

Statut : publiée en Latest stable [0.50.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.50.0), source `fb5f66b30ae30506dcd53f31ed03a5377260eb20`, le 2026-10-22. [Version précédente](RELEASE_0.49.0.fr.md).

## Changements

- Retrouvez 18 outils dans cinq familles consultables et filtrables depuis **Outils**. Mesures, bibliothèque d’annotations, table des quantités et superposition de révisions partagent **Réviser et mesurer**.
- Basculez le volet gauche entre Outils, Pages et Signets. La barre principale conserve une entrée Outils générale, sans raccourcis distincts pour les mesures et la bibliothèque.
- Utilisez quatre sections dans le panneau de mesure droit : Créer une mesure, Échelle de la page, Tracé et Données enregistrées. L’espace Mesures n’ajoute plus de barre horizontale au-dessus du PDF.
- Affichez un seul panneau contextuel à droite. Un brouillon de mesure actif bloque l’ouverture des autres panneaux ; quitter explicitement l’outil ou en changer annule le brouillon local et conserve les échelles confirmées.
- Naviguez entre les pages et réglez le zoom sous le PDF. Les valeurs telles que **149%** restent sur une ligne ; ajustements à la page/largeur et menu compact Options de lecture restent disponibles lorsque la fenêtre rétrécit.
- Distinguez rotation de lecture et rotation des pages PDF. Le rail droit ouvre les panneaux ; la zone d’état affiche les messages d’opération.
- Conservez l’édition des sommets enregistrés et ses régressions sur paquets publics. Les paquets publiés 0.49.0 rejoignent les régressions de mise à jour portable et MSI.

## Qualification

Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39854366778) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39854380539) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39854366334). 776 tests réussis, 11 ignorés par défaut. 1413 clés EN/FR. 42 scénarios portables. 36 références MSI. 180 journaux MSI. Aucune alerte Code Scanning ouverte. Six téléchargements publics et copies dist vérifiés par SHA256. 63 appels CLI de régression conservés vérifient insertion et suppression sur polylignes, périmètres et surfaces, nombres de sommets après réouverture, conservation des calibrations et quantités, recalcul des totaux, limites minimum/maximum et des types fixes, refus invalides/périmés et cycle de vie des mesures conservé. Ils ne constituent pas une qualification du bureau natif ni d’ARM64. 3 scènes de l’application et de ses pages PDFium rendues hors écran avec DX12 WARP, dont la disposition française à 720 × 480 et zoom 149%. Rapports, PNG et pixels décodés revérifiés par SHA256 ; fenêtres natives et périphériques physiques non qualifiés. [Preuves](qa/RELEASE_0.50.0.json).

## Périmètre

Cette version réorganise les parcours existants de lecture et d’outils ; elle n’introduit ni nouvelle commande CLI, ni nouveau type de mesure. Permissions, gardes d’aperçu/application et limites des mesures enregistrées restent applicables. AEC-002 et AEC-003 restent partiels.

Les preuves UI automatisées hors écran concernent uniquement les dispositions et interactions testées. Elles ne qualifient ni fenêtres natives, périphériques physiques, lecteurs d’écran ou ARM64 natif. Les paquets restent non signés ; les critères généraux 1.0 restent ouverts.
