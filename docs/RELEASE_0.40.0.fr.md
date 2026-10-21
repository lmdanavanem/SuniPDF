# SuniPDF 0.40.0

## Modifications

Annotations locales de longueur, polyligne, périmètre, surface polygonale simple et angle : calibration explicite sur deux points, unités métriques/impériales décimales, précision et données JSON. Chaque mesure conserve son échelle et sa géométrie ; aperçu, Annuler et copies CLI préservent les originaux. Tampons Stamp avec image intégrée, proportions et transparence. Filigranes texte/numérotation : vérification de toutes les pages et refus atomique des marges invalides ou débordements. Paquets 0.39.0 ajoutés aux migrations. Backlog AEC de 26 cartes donnant priorité au local ; cloud et collaboration reportés à la demande du propriétaire, tarifs ultérieurs.

[Mesures](MEASUREMENTS.fr.md) · [Tampons et décorations](PAGE_DECORATIONS.fr.md) · [Backlog AEC](AEC_BACKLOG.fr.md).

## Validation

La qualification finale sera enregistrée après les contrôles CI et des paquets sur la révision publiée.

## Limites

- Measurement vertices and calibration are entered as displayed-page coordinates in Rich annotations. Maximum 128 vertices and 0–4 decimal places; simple polygons only. Feet/inches are decimal units.
- Each annotation has its own calibration. Recreate it to change geometry or scale; generic move/resize/label edits are refused to prevent stale takeoffs. No page-wide recalibration, vector snapping, viewport scales, volume, symbol recognition or automatic counting is claimed.
- Measurement JSON records use a documented SuniPDF format; Bluebeam proprietary measurement compatibility and universal imported-metadata support are not claimed. The AEC backlog acceptance criteria remain open where incomplete.
- Image stamps are limited to 64 MiB encoded, 4096 pixels per side and four million pixels. Embedded pixels are independent of the source image after Apply. No digital-signature or identity verification is implied.
- Local tools remain free and open source. Cloud/collaboration implementation, deployment and billing are deferred; no prices are defined. Projects edits remain deferred.
- Windows packages remain unsigned without an independent publication key; ARM64 is cross-built only. Native hardware, printing, Narrator, installed updater interaction, broader hostile/customer corpus and PDF/A/PDF/UA conformance remain separate 1.0 qualification.
