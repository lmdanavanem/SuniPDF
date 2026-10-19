# Mentions des composants tiers

[English](THIRD_PARTY_NOTICES.md)

Chaque ZIP portable et MSI contient licenses/dependency-inventory.json et
licenses/dependency-licenses.txt : versions Rust verrouillées, révisions sources ou
empreintes des archives, expressions de licence et textes complets de licence,
copyright et NOTICE. Tout Cargo.lock est inventorié, y compris les outils de
construction et les autres plateformes. Les graphes Windows x64 et ARM64 sont indiqués
séparément, sans prétendre que chaque paquet ou police reste présent après optimisation.

OR indique un choix de licences ; AND des conditions cumulatives. Les anciennes
expressions avec une barre oblique restent inchangées. Les identifiants SPDX renvoient
aux textes fournis. Les copyrights, licences, NOTICE, attributions, restrictions de
nommage des polices et mentions de modification applicables doivent être conservés
selon ces textes. Un choix de licence n'impose pas toutes les autres possibilités.

Les archives Cargo et leurs mentions sont vérifiées contre Cargo.lock. Si un paquet
omet sa licence, un texte vérifié provient de sa révision source exacte, référencée
dans assets/licenses/upstream-sources.json. Une licence indisponible reste signalée et
bloque la génération si le paquet appartient à un graphe Windows. Le paquet macOS
dispatch est ainsi signalé et exclu des deux graphes Windows.

L’inventaire 0.33.0 actualise les versions et chemins sources d’AccessKit et d’egui/eframe.
L’objet VCS publié de huit composants egui 0.37.0 est indisponible en amont ; leurs
déclarations vérifiées par empreinte et l’octroi de licence du README egui publié sont
associés aux textes complets de la release officielle 0.37.0 épinglée. Chaque entrée
concernée signale cette exception de provenance, sans annoncer une identité complète
des sources. Les textes AccessKit et eframe proviennent de leurs révisions VCS exactes.

Le dossier licenses contient également :

- les notices natives originales de PDFium 151.0.7881.0, ses révisions et empreintes ;
- les textes complets et références des polices Ubuntu Light, Hack, Noto Emoji et
  emoji-icon-font ;
- les licences, auteurs, copyrights et provenances de Tesseract 5.5.3, Leptonica 1.87.0
  et des modèles anglais/français tessdata_fast verrouillés ;
- les mentions complètes de la bibliothèque standard Rust et de la chaîne réellement
  sélectionnée, avec la révision rustc ;
- la provenance du compilateur OCR, les empreintes des bibliothèques MSVC statiques
  et de l'Universal CRT, les conditions Microsoft Build Tools sélectionnées et la
  licence du SDK Windows installé ;
- les versions et textes réels des outils Rust/Cargo, CMake et Python, dont les
  exécutables ne sont pas redistribués ;
- la licence MS-RL de WiX 3 ; le MSI ajoute les versions et empreintes des exécutables
  candle/light réellement employés ;
- les licences MIT et Apache-2.0 de SuniPDF.

Les empreintes identifient les entrées du compilateur OCR, sans attestation indépendante
de l'édition de liens. Les anciens caches sans ces preuves doivent être reconstruits.
PDFium est un binaire amont verrouillé : ses notices, version et empreinte sont
vérifiées sans prétendre connaître le compilateur privé de son producteur. Les DLL
système utilisées par les API Windows/DX12 ne sont pas redistribuées.

La préparation portable vérifie le dossier puis le ZIP. La préparation MSI vérifie
son entrée, ajoute les preuves WiX dans une copie distincte, puis extrait le MSI pour
vérifier les exécutables et licences sans installer l'application. Un fichier manquant,
modifié, ajouté ou une entrée ZIP ambiguë bloque la validation. Le README propre au
MSI est exclu de cette comparaison.

L'icône provient du visuel fourni par l'auteur assets/icon/sunipdf-source.png ; ses
formats sont produits par make_icon.py. Aucun nouveau visuel tiers n'est ajouté. Les
ressources supplémentaires nécessitent une vérification de leurs conditions réelles.
