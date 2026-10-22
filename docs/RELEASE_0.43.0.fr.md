# SuniPDF 0.43.0 — échelles de page enregistrées et inspection des mesures

[English](RELEASE_0.43.0.md) · [Guide des mesures](MEASUREMENTS.fr.md)

État : en préparation, non publiée.

Enregistrez une échelle confirmée dans le PDF pour la retrouver à la réouverture du document. Enregistrer ou retirer une échelle de page constitue une modification explicite, avec Annuler/Rétablir ; enregistrez ensuite le PDF pour conserver le changement sur disque. L'échelle de page enregistrée est distincte de la calibration temporaire et de l'échelle propre à chaque mesure existante. Recalibrer les prochaines mesures ne modifie jamais silencieusement les quantités précédentes.

L'espace Mesures liste les mesures enregistrées de la page courante, leurs valeurs, unités et calibrations. Retrouvez une mesure sur la page ou réutilisez explicitement son échelle physique pour les prochaines mesures. Un angle ne fournit pas d'échelle physique. Consulter la liste ne modifie pas le PDF. Les métadonnées incompatibles sont signalées.

La CLI ajoute `page-calibration`, `set-page-calibration` et `remove-page-calibration`. `measure --use-page-calibration` utilise explicitement l'échelle enregistrée, convertie dans l'unité demandée. Les modifications produisent une nouvelle copie PDF, préservent les protections compatibles et refusent d'écraser une destination existante ou l'original. Enregistrer l'échelle exige le droit de modifier le document ; un PDF autorisant seulement les annotations conserve la calibration temporaire pour ajouter les mesures permises.

Les échelles utilisent un format de métadonnées ouvert, versionné et borné, en coordonnées PDF. La rotation conserve l'échelle physique ; un rognage ou une géométrie incompatible est refusé. Modifications de pages, changements de document et réponses tardives du moteur ne peuvent pas transférer silencieusement une échelle à une autre page ni remplacer une calibration locale plus récente. Les annotations existantes gardent leur géométrie et leurs valeurs.

Les paquets ZIP et MSI publiés en 0.42.0 rejoignent les tests de mise à jour. Les résultats seront enregistrés après les contrôles.

Modification des sommets enregistrés, recalcul global, accrochage vectoriel, vues multi-échelles et qualification sur des plans réels restent ouverts. Le local demeure gratuit et open source. Les paquets Windows restent non signés. ARM64 est compilé en cible croisée ; le matériel ARM64 natif et la précision physique ne sont pas qualifiés par les tests automatisés.
