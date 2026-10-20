# Raccourcis clavier

Ouvrez **Aide → Raccourcis clavier**, également accessible dans la palette de commandes. Filtrez la liste, choisissez une commande, saisissez par exemple `Ctrl+Shift+J`, puis **Attribuer**. **Enregistrer** applique le brouillon complet et sauvegarde les préférences atomiquement. **Annuler** ou Échap l’abandonne.

**Désactiver** retire tous les raccourcis de la commande, variantes comprises. **Rétablir cette commande** restaure ses touches d’origine ; **Tout rétablir** efface les personnalisations. Les conflits sont refusés : désactivez ou réattribuez d’abord l’autre commande. Un rétablissement qui crée un conflit doit être corrigé avant enregistrement. Menus, palette, accueil et infobulles suivent les raccourcis actifs.

Cette première version personnalise les commandes déjà présentes dans la table de raccourcis. Une personnalisation remplace toutes les variantes d’une commande ; les macros et commandes auparavant sans raccourci restent hors périmètre.

Combinaisons : `Ctrl`, éventuellement `Shift`, puis une lettre, un chiffre, F2–F12, Tab, Delete, Plus, Equals, Minus, OpenBracket ou CloseBracket. Presse-papiers, tout sélectionner, annuler/rétablir du texte, Ctrl+F4/F6 et Ctrl+Shift+Delete restent réservés. Navigation, Échap, F5, F11, historique Alt+flèches, déplacement d’objets et touches Windows conservent leur fonctionnement dédié. `Plus` accepte les claviers qui signalent Shift pour cette touche. Alt/AltGr n’est pas intercepté.

Dans un champ de saisie, seuls Ouvrir, Enregistrer, Enregistrer sous, Imprimer, Rechercher, Formulaires et Palette utilisent leurs raccourcis Ctrl. Saisie et annuler/rétablir restent au champ. Les autres dialogues suspendent les commandes ; une palette focalisée accepte seulement son propre basculement.

Les préférences `%APPDATA%\SuniPDF\settings.json` stockent des identifiants stables dans `shortcuts`, avec une chaîne de combinaison ou `null` pour désactiver. Les anciennes préférences gardent les valeurs par défaut. Identifiant inconnu, syntaxe invalide ou conflit désactivent toutes les personnalisations ; l’éditeur invite à rétablir les valeurs par défaut. Un échec d’écriture conserve les réglages actifs.

Les tests egui sans fenêtre contrôlent entrées, conflits, focus, affichage, annulation et sérialisation. Claviers Windows natifs, Narrateur et matériel restent à qualifier. L’intégration Explorer reste distincte.
