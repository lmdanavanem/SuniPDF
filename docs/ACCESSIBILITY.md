# Accessibilité — état réel

SuniPDF expose son interface aux technologies d'assistance par AccessKit (egui). Ce document décrit ce qui est
nommé et testable au clavier, et ce qui ne l'est pas. **Vérification manuelle avec le Narrateur Windows : NON
FAITE** (aucune session de test avec un lecteur d'écran n'a été exécutée sur ce poste).

## Principe commun

- Tout bouton dessiné à la main porte un libellé AccessKit lisible (`widget_info`), jamais un glyphe de la police
  d'icônes (`workspace::bar_button`, `shell::tool_row`).
- Les raccourcis à une lettre sont coupés dès qu'un champ de saisie a le clavier ; Échap suit un ordre unique
  (`workspace::EscState`).
- Le bit 10 (extraction pour l'accessibilité) est toujours accordé par Protéger (D050).

## Formulaires AcroForm (tranche forms)

| Contrôle | Nom accessible (clé) | Clavier |
|---|---|---|
| Rectangle de champ sur la page (outil Remplir) | `a11y.forms.widget` (« Champ {name}, type {kind} ») ou `a11y.forms.widget_value` | Clic seulement ; pas d'activation par Espace / Entrée (feuille de route) |
| Éditeur posé sur un champ texte | `a11y.forms.editor` (« Saisie du champ {name} ») | Entrée valide (Ctrl+Entrée en multiligne), Échap annule, Tab / Maj+Tab valident et passent au champ texte ou liste suivant / précédent (contenu sélectionné) |
| Liste de choix flottante | `a11y.forms.option` (« {name} : {option} ») par entrée | Clic, Échap ferme |
| Panneau « Champs du formulaire » (Ctrl+Maj+F) | titre `forms.panel.title`, un contrôle egui natif par champ (zone de texte, case `a11y.forms.checked` / `unchecked`, boutons radio, liste déroulante, cases de zone de liste), chacun rattaché au libellé du champ par `labelled_by` | Tab entre contrôles, Espace, flèches, Entrée ; Échap restaure la saisie d'une zone de texte |
| Bandeaux (formulaire présent, XFA, scripts), boîte Aplatir | textes traduits, `dialog::show` (Entrée / Échap) | Tab |

Limites : la page est une image (texture) ; son texte n'est pas exposé aux lecteurs d'écran, seuls les rectangles
de champs et le panneau le sont. Lecture de la structure balisée (Tagged PDF) : hors périmètre. Contraste des
surbrillances : `theme::BLUE` / `theme::ERROR` à 12 % de remplissage et contour à 100 % (22 % et 1,5 px au survol),
vérifié sur capture en thème clair et sombre (pas de mesure automatique du ratio pour ces contours).
