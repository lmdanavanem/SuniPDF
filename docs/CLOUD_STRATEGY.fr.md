# SuniPDF local gratuit et futur cloud d'entreprise

[English](CLOUD_STRATEGY.md) · [Backlog bâtiment](AEC_BACKLOG.fr.md)

## Décision du propriétaire, 21 octobre 2026

Le cloud et la collaboration sont reportés. Développer et qualifier d'abord les fonctions locales PDF et bâtiment. La 0.40.0 n'inclut aucun service cloud, connecteur, facturation ou déploiement public. Ce document décrit une orientation future, sans annoncer une implémentation disponible.

## Modèle retenu

L'application locale reste gratuite et open source sous la licence actuelle MIT OR Apache-2.0. Édition, OCR, formulaires, signatures, récupération et opérations locales restent disponibles sans compte cloud ni abonnement.

L'offre commerciale future sera l'hébergement géré et la collaboration facultative en entreprise : espaces, PDF communs, droits, versions immuables, stockage et exploitation. Les tarifs et conditions seront définis plus tard. Aucun prix, forfait ou encaissement n'est supposé. La licence du dépôt reste inchangée ; celle du service sera réexaminée avant son lancement commercial.

## Lots reportés

1. Isolation des entreprises, rôles lecteur/éditeur/administrateur, versions PDF immuables et réservations. Refuser les conflits en conservant anciennes versions et originaux locaux.
2. Connecteur facultatif : transferts explicites, téléchargements dans de nouvelles copies, SHA256 et protection des identifiants. Les outils locaux restent indépendants.
3. Hébergement nommé : HTTPS, OIDC/SSO, secrets, quotas, audit, supervision, sauvegarde/restauration testées et procédure d'incident. Les PDF restent exportables.
4. Annotations simultanées sur une base immuable : événements authentifiés, versions par annotation, idempotence, reconnexion et droits vérifiés sur plusieurs clients. Une réservation ne constitue pas une revue simultanée.
5. Facturation après décision sur tarifs, taxes, droits et cycle d'abonnement. Notifications authentifiées et idempotentes, sans désactivation locale ni suppression silencieuse.

AEC-020 et AEC-021 restent reportés dans le [backlog de 26 cartes](AEC_BACKLOG.fr.md). Projects reste différé ; le backlog du dépôt sert de référence actuelle.
