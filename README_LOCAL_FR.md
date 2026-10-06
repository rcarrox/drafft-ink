# DrafftInk Local 0.3.0

Version personnelle du tableau DrafftInk, sans collaboration reseau.

## Utilisation sous Windows

Le ZIP **DrafftInk_Windows_Portable_0.3.0.zip** produit par GitHub Actions est deja compile.
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

- `DrafftInk-Windows-Portable-0.3.0` : version locale Windows precompilee ;
- `DrafftInk-Web-0.3.0` : fichiers statiques a deposer sur un serveur web.

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


## Nouveautes 0.3.0

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
