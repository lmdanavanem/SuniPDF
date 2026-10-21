# SuniPDF 0.42.0 — mesures directement sur la page

[English](RELEASE_0.42.0.md) · [Guide des mesures](MEASUREMENTS.fr.md)

État : intégration terminée ; qualification de la release en cours.

Les mesures disposent d’un bouton libellé à côté de Tous les outils et d’un espace dédié. Étalonnez une page avec deux clics et une distance connue, puis tracez longueurs, polylignes, périmètres, surfaces et angles directement sur le PDF. Un aperçu sur le plan affiche la géométrie et le résultat ; Maj contraint un segment horizontalement ou verticalement. Retour arrière retire le dernier point et Échap annule le tracé.

Les longueurs et angles ouvrent l’aperçu PDF au dernier point. Pour une polyligne ou un polygone, utilisez Entrée, le double-clic ou Prévisualiser la mesure. Appliquer crée l’annotation réelle ; Annuler conserve le document intact. L’enregistrement conserve géométrie, étalonnage et valeur dans le PDF ; Annuler la dernière modification les retire ensemble. L’outil reste actif pour tracer d’autres mesures.

Les échelles sont propres à chaque page pendant la session. Changer d’unité convertit l’étalonnage. Les modifications ordinaires d’annotations conservent les échelles confirmées ; une modification de géométrie ou de structure des pages les invalide. Un tracé inachevé ne peut pas changer silencieusement de page, document ou outil.

Les contrôles automatisés couvrent la visibilité du bouton en français et en anglais, les clics egui, l’étalonnage, l’aperçu/application, les doubles-clics, le clavier et l’isolation des pages. Les résultats seront consignés à la fin des vérifications.

L’échelle de session n’est pas une propriété partagée enregistrée sur la page. Rouvrir un document ou restaurer un onglet suspendu demande un nouvel étalonnage pour les nouvelles mesures ; les annotations enregistrées gardent leur propre échelle. L’édition des sommets enregistrés, l’accrochage aux vecteurs, les zones à échelles multiples et le recalcul global restent au backlog. Cette version ne revendique pas une compatibilité complète avec Bluebeam. Les fonctions locales restent gratuites et open source ; le cloud collaboratif reste différé.

Les paquets Windows restent non signés. ARM64 est compilé sur une autre architecture ; les tests automatisés ne qualifient ni le matériel ARM64 ni la précision d’un relevé physique.
