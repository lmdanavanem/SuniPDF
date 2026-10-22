# SuniPDF 0.42.0 — mesures directement sur la page

[English](RELEASE_0.42.0.md) · [Guide des mesures](MEASUREMENTS.fr.md)

État : publiée en Latest stable [0.42.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.42.0), source `96ad768b24813e19c01de2881d3cb53c5b2fdc6c`.

Les mesures disposent d’un bouton libellé à côté de Tous les outils et d’un espace dédié. Étalonnez une page avec deux clics et une distance connue, puis tracez longueurs, polylignes, périmètres, surfaces et angles directement sur le PDF. Un aperçu sur le plan affiche la géométrie et le résultat ; Maj contraint un segment horizontalement ou verticalement. Retour arrière retire le dernier point et Échap annule le tracé.

Les longueurs et angles ouvrent l’aperçu PDF au dernier point. Pour une polyligne ou un polygone, utilisez Entrée, le double-clic ou Prévisualiser la mesure. Appliquer crée l’annotation réelle ; Annuler conserve le document intact. L’enregistrement conserve géométrie, étalonnage et valeur dans le PDF ; Annuler la dernière modification les retire ensemble. L’outil reste actif pour tracer d’autres mesures.

Les échelles sont propres à chaque page pendant la session. Changer d’unité convertit l’étalonnage. Les modifications ordinaires d’annotations conservent les échelles confirmées ; une modification de géométrie ou de structure des pages les invalide. Un tracé inachevé ne peut pas changer silencieusement de page, document ou outil.

Les contrôles automatisés couvrent la visibilité du bouton en français et en anglais, les clics egui, l’étalonnage, l’aperçu/application, les doubles-clics, le clavier et l’isolation des pages. Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696093211) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696104747) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39696092839). 662 tests réussis, 10 ignorés par défaut. 1224 clés EN/FR. 34 scénarios portables. 28 références MSI / 140 journaux. Aucune alerte Code Scanning ouverte. Six téléchargements publics sans authentification et copies dist vérifiés par SHA256 ; quatre appels CLI du paquet vérifient version, aide, création d’une longueur de 10 m et réouverture sans modifier l’original.

L’échelle de session n’est pas une propriété partagée enregistrée sur la page. Le changement d’onglet conserve l’échelle de chaque document. Fermer puis rouvrir un document demande un nouvel étalonnage pour les nouvelles mesures ; les annotations enregistrées gardent leur propre échelle. L’édition des sommets enregistrés, l’accrochage aux vecteurs, les zones à échelles multiples et le recalcul global restent au backlog. Les fonctions locales restent gratuites et open source.

Les paquets Windows restent non signés. ARM64 est compilé sur une autre architecture ; les tests automatisés ne qualifient ni le matériel ARM64 ni la précision d’un relevé physique.

[Preuves de publication et empreintes des six fichiers](qa/RELEASE_0.42.0.json).
