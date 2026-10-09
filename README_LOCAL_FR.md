# Qurso🌿 0.27.0

Version personnelle du tableau DrafftInk, sans collaboration reseau.

## Utilisation sous Windows

Le ZIP **DrafftInk_Windows_Portable_0.27.0.zip** produit par GitHub Actions est deja compile.
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

- `DrafftInk-Windows-Portable-0.27.0` : version locale Windows precompilee ;
- `DrafftInk-Web-0.27.0` : fichiers statiques a deposer sur un serveur web.

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

## Nouveautes 0.20.0

Dans Text, taper sum(, prod(, int(, lim(, sqrt( ou frac( cree immediatement un bloc math et ouvre un mini editeur de code. Le symbole et ses arguments sont actualises sur la feuille pendant la frappe. Entree valide et continue le texte ; Escape valide et termine l'edition Text. Selectionner un bloc de code puis Ctrl+Entree (ou double clic sur le bloc deja en edition) permet de le modifier. Les champs 0.8 restent disponibles.

Exemples : sum(kx,k,1,n), prod(k,k,1,n), int(x²,x,1,2), lim(x,x,5), sqrt(x²), frac(a,b), frac(a,frac(b,c)). La source reste dans le JSON avec le bloc ; les arguments manquants du rendu provisoire sont des points. Le code saisi n'est pas remplace par une chaine normalisee pendant la frappe, pour conserver le caret.

## Nouveautés 0.10.0

Poignées traversantes pour les objets, y compris textes/formules en miroir et objets tournés. Axe des formules aligné sur le signe égal. Double-clic sur un bloc validé pour rouvrir son code ; Ctrl+Entrée reste disponible. Le mini panneau suit la formule, 10 pixels sous le bloc.

Pour rééditer une formule validée : sélectionner l’outil Select puis double-cliquer directement sur le bloc de formule. Le panneau de code se rouvre sous ce bloc.

## Nouveautés 0.11.0 — mémoire

Cache d’images décodées limité à 32 Mio, libération des caches des canvas inactifs, aperçus adaptés au zoom (2048 pixels maximum sur le grand côté). Les données originales restent dans le document et sont utilisées pour l’export ; l’affichage temporaire réduit peut être moins détaillé à fort zoom pour une source supérieure à 2048 pixels. La texture d’écran et les buffers de scène sont réutilisés. L’export partage le moteur GPU existant. Les variantes anciennes de traits sont supprimées et le cache de géométrie est plafonné à environ 4 Mio. L’historique garde jusqu’à 50 états, avec un budget de charges clonées de 16 Mio par canvas (au moins le dernier état Undo/Redo conservé, même s’il dépasse ce budget). Le passage entre canvas déplace le document plutôt que de cloner son historique.

Ces budgets concernent des composants, pas toute la RAM du navigateur. Le seuil global de 200 Mo n’est pas confirmé. Lire docs/MEMORY_0.11.md pour les mesures et limites.

## Nouveautés 0.12.0

Flèche seule : déplacement fin d’un pixel de rendu ; Shift+flèche : pas historique de 20 unités, avec répétition tant que la touche est maintenue. Ctrl+haut/bas applique un style exposant/indice aux caractères originaux, sans alphabet Unicode limité. La famille de police choisie pour Text fournit aussi les lettres/chiffres des formules ; la police math reste le secours pour les symboles structurés.

Ctrl+S enregistre un PNG complet du canvas (même pendant Text/Math) ; Ctrl+Shift+S conserve le JSON. La sauvegarde régulière garde sa récupération navigateur et écrit aussi le PNG dans le dossier d’export autorisé, seulement après modification. Sans permission, pas de rafale de téléchargements : réautoriser le dossier dans Settings. Le PNG contient les données éditables du document. Le statut de sauvegarde est visible.

Zoom au pavé tactile : vitesse 2× par défaut, réglable de 0,25× à 8× dans Settings ; 1× retrouve l’ancien comportement. Pan deux doigts conservé. Panneau Stroke resserré.

## Nouveautés 0.16.0

- Text/Math : proportions conservées par défaut, Shift pour déformer librement.
- Poignées circulaires plus petites, marge autour des caractères ; retournement et rotation conservés.
- `bin(n,k)` dans Text, y compris arguments imbriqués et aperçu pendant la saisie.
- Bornes infinies Unicode dans le code LaTeX et correction des touches AltGr françaises.
- Math : icône TeX fournie, GelPen, champ plus aéré, copier/coller au curseur avec sélection.

## Nouveautés 0.17.0

- Settings : couleur des boutons actifs, styles de trait sélectionnés et poignées/cadre de sélection, enregistrée avec les préférences.
- Le rectangle de sélection continue sur les panneaux et menus jusqu’au relâchement, sans activer leurs boutons.

## Nouveautés 0.20.0

Cache hors connexion des fichiers de l’application et mises à jour explicites, limitées au dossier hébergé. Pour le déploiement FTP, lire README_WEB_FR.md. Documents, préférences, polices et rendu existants conservés.


## Nouveautés 0.27.0

- Menu Insérer dans la barre : images et PDF (pages rendues en images éditables, jusqu’à 20 pages par import), repère cartésien, cercle trigonométrique, cube 3D, plan géométrique/complexe.
- Courbe vectorielle : saisie de y=f(x), échantillonnage dans x∈[-10,10], outils sin/cos/tan/sqrt/abs/ln/log/exp ; la courbe et les axes sont des objets vectoriels éditables.
- Chronomètre flottant redimensionnable et déplaçable, indépendant du zoom/pan, commandes démarrer/pause/stop/reset, sans tours.
- Draw et Highlighter continuent le trait en faisant défiler automatiquement la feuille au bord de la fenêtre.
- PDF.js est chargé seulement à l’import et intégré au cache hors connexion ; les pages rasterisées sont limitées pour contenir la mémoire.
- Favicon fourni par l’utilisateur, statut « Enregistré » en blanc et diagnostic distinctif du cache hors connexion (réseau, HTTP ou empreinte incohérente).
