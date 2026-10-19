# Inspecter un PDF enregistré

[English](PDF_INSPECTION.md)

Ouvrir **Fichier → Propriétés** (`Ctrl+D`), développer **Inspecter le PDF enregistré**, puis choisir
**Inspecter**. Enregistrer d’abord les modifications du document. Le rapport porte sur la version
enregistrée dont le SHA256 est affiché. Si le fichier change sur le disque, le recharger avant de relancer
l’inspection. Les brouillons de métadonnées du dialogue ne sont pas appliqués. Fermer, changer d’onglet
ou modifier le document invalide les réponses en attente.

L’inventaire repère les déclarations de JavaScript, actions automatiques, actions externes, fichiers
embarqués, XFA, multimédia et signatures. Les compteurs portent sur les dictionnaires correspondants,
y compris les objets inutilisés ; ils ne comptent pas les pièces jointes, scripts ou signatures uniques.
Le texte des contenus et les noms des pièces jointes ne sont pas exposés. Aucune action n’est exécutée
et aucune copie du document n’est écrite.

```powershell
sunipdf-cli inspect document.pdf
sunipdf-cli inspect protected.pdf --password-env PDF_PASSWORD
```

La CLI écrit du JSON sur stdout. Un inventaire terminé renvoie `0`, même si des éléments sont détectés.
Un inventaire partiel conserve les résultats et renvoie `2` ; `limitations` indique les références non
résolues, noms de fonctions malformés ou limites de parcours. Identifiants absents : `3` ; PDF invalide :
`4` ; erreur de lecture : `5`. Les autres refus utilisent le JSON d’erreur habituel sur stderr.
Les sources de mot de passe par variable d’environnement et entrée standard gardent leurs protections.

Le rapport contient SHA256, taille, nombre d’objets stockés et de valeurs parcourues, état de complétude
et sept compteurs. Limites : 128 Mio d’entrée, 250 000 objets, un million de valeurs et 32 niveaux de
dictionnaires/tableaux imbriqués. Les références ne sont pas suivies récursivement : chaque objet stocké
est inspecté séparément et la résolution des noms indirects est bornée. Ces bornes d’inventaire après
analyse du PDF ne garantissent ni la mémoire ni la durée de son parseur natif.

Cet inventaire structurel n’est ni un antivirus, ni une garantie d’innocuité, ni un contrôle d’intégrité
des signatures, ni un validateur PDF/A/PDF/UA. Un résultat vide ne prouve pas l’absence de risque.
Le contenu des scripts, pièces jointes et autres flux n’est pas inspecté ; le parseur peut décoder les
flux d’objets et de références PDF. Vérification des signatures et nettoyage explicite restent séparés.

La garde de version enregistrée, partagée avec la vérification des signatures, contrôle la taille puis
calcule l’empreinte du fichier par blocs. Elle refuse les fichiers modifiés et conserve la version capturée
comme source de l’opération, sans allouer un second tampon de la taille du fichier.

Le contrôle indépendant de la CLI empaquetée est `scripts/test-pdf-inspection.py` : quatorze scénarios
sur des PDF/xref écrits indépendamment, références absentes/cycliques, flux opaques, erreurs structurées,
fichiers trop volumineux, AES-128/256 et identifiants par environnement. Interface native, Narrator et
corpus hostile élargi restent à qualifier séparément.
