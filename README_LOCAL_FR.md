# DrafftInk Local 0.6.0

Version personnelle du tableau DrafftInk, sans collaboration reseau.

## Utilisation sous Windows

Le ZIP **DrafftInk_Windows_Portable_0.6.0.zip** produit par GitHub Actions est deja compile.
Il ne faut installer ni Rust, ni wasm-pack, ni Visual Studio.

1. Decompresser le ZIP.
2. Double-cliquer sur `Lancer DrafftInk.cmd`.
3. Choisir Chrome, Edge ou le navigateur par defaut.

Des lanceurs directs `Ouvrir avec Edge.cmd` et `Ouvrir avec Chrome.cmd` sont aussi fournis.
`Arreter DrafftInk.cmd` ferme le petit serveur local.

Le serveur local ecoute seulement sur `127.0.0.1:8765` et sert les fichiers statiques du dossier `web`.

## Compilation

La compilation est faite par GitHub Actions, sur Ubuntu, avec Rust 1.91.1 et la cible
`wasm32-unknown-unknown`. Le workflow telecharge le binaire officiel wasm-pack 0.15.0
et verifie son SHA-256 avant utilisation.

Workflow : `.github/workflows/build-portable.yml`.

A chaque push sur `main`, deux artefacts sont produits :

- `DrafftInk-Windows-Portable-0.6.0` : version locale Windows precompilee ;
- `DrafftInk-Web-0.6.0` : fichiers statiques a deposer sur un serveur web.

## Modifications de cette branche

- serveur collaboratif retire du workspace ;
- dependances Loro/WebSocket/tungstenite retirees ;
- interface de collaboration masquee ;
- URL de salles ignorees ;
- autosauvegarde locale conservee ;
- glissement a deux doigts : deplacement du tableau ;
- pincement du pave tactile : zoom fluide autour du pointeur ;
- Espace maintenu : main/deplacement temporaire, puis retour a l'outil precedent ;
- `+` / `-` : zoom ;
- `0` : zoom 100 % ;
- `F` : ajuster la selection ou le dessin a l'ecran ;
- `K` : surligneur ;
- aide des raccourcis completee.

## Navigateur

Chrome et Edge recents sont les cibles prioritaires car DrafftInk utilise WebGPU.


## Nouveautes 0.4.0

- reglages persistants depuis le menu Settings ;
- raccourcis des outils personnalisables ;
- document d'introduction JSON personnalisable (vide = aucune intro) ;
- dossier d'export par defaut pour Chrome/Edge, avec repli sur Telechargements ;
- sauvegarde automatique activable/desactivable et intervalle configurable ;
- restauration de la derniere feuille configurable ;
- pave tactile : glissement a deux doigts pour deplacer, pincement pour zoomer/dezoomer autour des doigts ;
- formules mathematiques editees directement sur la feuille, avec rendu en direct ;
- saisie mathematique simplifiee type Maple/GeoGebra (sqrt(x), (a+b)/(c+d), int(...), sum(...), vec(...)) tout en conservant LaTeX ;
- Echap annule l'edition mathematique et restaure/supprime la formule en cours.


## Nouveautes 0.4.0

- panneau Properties masquable pour les outils depuis Settings (masque par defaut) ;
- Draw demarre en Calligraphy avec largeur uniforme ;
- Noto Sans reactive et XITS Symbols ajoute pour une meilleure couverture Unicode ;
- acces aux polices installees sur le PC via l'API Local Font Access de Chrome/Edge ;
- exposants/indices Unicode issus de raccourcis texte (ex. x⁴) normalises proprement ;
- saisie mathematique structuree : / ouvre une fraction, ^ un exposant, _ un indice ;
- Espace ou Fleche droite sort d'un niveau de fraction/exposant/indice ;
- fractions imbriquees possibles sans parenthese, avec un espace par niveau de sortie.


## Nouveautes 0.6.0

- champs de raccourcis Settings lisibles : texte noir sur fond blanc ;
- raccourcis par defaut : D = Draw, M = Math, H = Pan ;
- ^ fonctionne comme entree d'exposant, y compris avec les touches mortes des claviers francais ;
- raccourcis texte : Ctrl+Fleche haut = exposant, Ctrl+Fleche bas = indice ; Espace ou Fleche droite sort du mode ;
- exposants et indices texte restent dans le meme objet texte et la meme famille de police ;
- Properties du texte reste visible pendant l'outil Text mais disparait lors d'un changement d'outil ;
- ajout de plusieurs Canvas : + Canvas, fermeture, renommage au double-clic ou avec le crayon ;
- Architect devient le style par defaut des formes geometriques ;
- Shift + Ellipse = cercle, Shift + Rectangle = carre, Shift + Line/Arrow = angle snap.

## Nouveautes 0.6.0

- Math : caret noir clignotant, exposant `^` (touche morte francaise comprise), indice `_`, raccourcis Ctrl+haut/bas sans deplacement du curseur.
- Escape conserve le texte ou la formule et termine l'edition.
- Properties : Solid, Dashed 1 (long), Dashed 2 (court), Dotted sous Stroke width.
- Images tournees : resize dans le repere local, coin oppose fixe.
- Ctrl + glisser une poignee d'image : rognage non destructif, y compris apres rotation. Relacher Ctrl et glisser pour redimensionner. Undo/Redo et JSON conservent le rognage et les pixels source.
