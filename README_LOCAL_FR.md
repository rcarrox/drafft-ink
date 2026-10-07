# DrafftInk Local 0.8.0

Version personnelle du tableau DrafftInk, sans collaboration reseau.

## Utilisation sous Windows

Le ZIP **DrafftInk_Windows_Portable_0.8.0.zip** produit par GitHub Actions est deja compile.
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

- `DrafftInk-Windows-Portable-0.8.0` : version locale Windows precompilee ;
- `DrafftInk-Web-0.8.0` : fichiers statiques a deposer sur un serveur web.

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


## Nouveautes 0.5.0

- champs de raccourcis Settings lisibles : texte noir sur fond blanc ;
- raccourcis par defaut : D = Draw, M = Math, H = Pan ;
- Dans Math, ^ ouvre un exposant ; depuis la 0.8, Text conserve ^ litteralement, touche morte francaise comprise ;
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

## Nouveautes 0.7.0

- La police choisie est conservee pour les prochains textes, y compris apres redemarrage. Settings permet de choisir la police par defaut (Google Sans Medium initialement).
- Le lanceur Windows rend automatiquement disponibles les polices installees sur le PC, via son serveur loopback existant. Settings permet de choisir la police par defaut et d'actualiser la liste. La version web statique utilise Local Font Access si necessaire. Le bouton "Polices installees sur le PC" disparait des Properties Text. Les fontes sont mises en cache uniquement sur le PC, sans redistribution.
- Google Sans Medium est utilise pour les lettres/chiffres Math et pour son champ de saisie ; les symboles mathematiques absents utilisent XITS. Si la police locale est absente ou inaccessible, Noto Sans sert de secours.
- Clic droit sur Laser : palette de couleur. La couleur est memorisee.
- Panneaux Outils, Properties, couleurs/traits, zoom et palettes deplacables ; les positions des panneaux principaux sont memorisees et peuvent etre reinitialisees dans Settings.
- Panneau couleurs/traits sur deux colonnes et deux rangees, avec quatre styles de contour illustres.
- Curseurs SVG personnalises : fleche blanche avec ombre/contour configurable et curseur Text/Math.
- Poignees au centre des quatre bords : resize sur un seul axe, y compris apres rotation. Le crop Ctrl des images fonctionne aussi avec les poignees des bords. Text/Math conservent leur geometrie et leur source lors du redimensionnement.
- Optimisations : sources d'images partagees entre duplications et Undo/Redo, decode unique par source, nettoyage des caches supprimes, cache des formules et du cmap des glyphes, animations du caret planifiees au lieu d'un rendu continu. Les pixels source restent intacts et l'export conserve sa resolution.

## Nouveautes 0.8.0

- Text conserve les caracteres saisis, dont ^ et les sequences composees du clavier francais. Ctrl+fleche haut/bas reste reserve aux exposants/indices. Les remplacements Unicode externes (^4, ^>, ^<) ne consomment plus le caractere qui precede leur declencheur.
- Gras, italique et soulignement d'une selection avec Ctrl+B, Ctrl+I, Ctrl+U. La mise en forme est conservee dans le JSON et dans les rendus/export PNG. Le gras/italique peut etre synthetise si une seule variante de la police locale est disponible.
- Text accepte des formules dans son flux : Fraction, Racine, Racine n-ieme, Somme, Produit, Integrale et Limite. Les champs restent accessibles depuis Properties ; les expressions de type 1/2, sqrt(x), x^2 et fractions imbriquees sont traduites sans imposer LaTeX. Selectionner le bloc puis cliquer sa commande pour le modifier. Les anciens objets Math restent lisibles/editables.
- Symboles Σ, ∏, ∫, lim, ≥, ≤ et ∞ dans Text. Noto Sans/STIX Two Math completent les glyphes absents de la police choisie. Une police locale non encore chargee ne rend plus le texte invisible.
- Une poignee d'image peut traverser le bord oppose pour retourner l'image horizontalement et/ou verticalement. La rotation commence aussi juste a l'exterieur d'un coin, autour du centre et sans saut d'angle. Ctrl+poignee continue de rogner, meme apres miroir/rotation. Pixels originaux et Undo/Redo preserves.
- Ctrl+P cache/retablit outils, panneaux et onglets pour la presentation. F11 entre/sort du plein ecran (navigateur Chrome/Edge).
- Le cache de rendu Text conserve aussi les decorations et formules, sans reconstruire les textes termines a chaque clignotement.
- CI ajoute un test Chromium sur le WASM reel avec captures : saisie Unicode/remplacements, selection/mise en forme, fractions/racines, presentation/plein ecran, miroirs, rotation et Undo/Redo. Un clavier francais physique et Beeftext sous Windows restent distincts de ces simulations.

- Export PNG : les donnees du document sont embarquees en UTF-8 (iTXt) pour conserver les symboles, blocs de formule et noms Unicode ; les anciens PNG zTXt restent importables.
