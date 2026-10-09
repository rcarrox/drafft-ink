# Reprise — 0.12.0

8 octobre 2026. Branche codex/0.12.0-text-save-controls ; lire sa PR pour validation/fusion/artefact exacts. Travail local autorisé, Rust/WASM exclusivement Actions.

Demandes : nudge fin 1 pixel vs Shift pas20 historique et répétition ; vrais styles exposant/indice sur tout caractère de Text ; font parent appliquée aux chiffres/lettres des formules inline ; Ctrl+S PNG complet et autoPNG périodique dans dossier autorisé ; vitesse pinch2× configurable ; Stroke plus compact. EXE uniquement une question : ne pas construire/remplacer Chrome/Edge sans demande. Une version native pourrait réduire le coût navigateur mais doit conserver fonts/touchpad et être mesurée ; wrapperElectron conserveChromium.

En validation. SourceAuto précédente écrivait uniquementIndexedDB ; garder récupération maisajouterPNG.Ctrl+Shift+S JSON. Autosave sanspermission ne doit pas télécharger ni demanderpermission enarrière-plan. Statutvisible,récupérationaprèséchec,unchangedskip. Les caractères restentinchangés.JSONscriptdefault0compatible. Préservermémoire0.11,pinchpan,Beeftext,cropmirrorUndo et math. Pas deprivatefonts publiées. TestsGPUlogicielCIlimités,ne pasaffirmerpixelsvalides surcaptureuniforme.

Dernière livraison0.11 : mainb7c6440e1bedee6e90e77f34aac4dd061eb3ff6e, PR12, CI37715815143/Portable37715815060 réussis. ZipSHA2565d42480cd9c552d0df647409aac5bcfbfbdc0e6255148ec55827678de0556233. 200MoEdgeTotalnonvérifié ; WASM21%de moins cas2images.

IMPORTANTOUTILS : ne jamais réessayer Cua.waitForEvent(download) ; a bloqué20958secondes. TéléchargerlesartifactsviaGitHub et testerfilesystemavecVM/mocks/CI plutôtqu’attendreunévénementdedownloadducontrôleurnavigateur. Usertab8887 àpréserver.

## Reprise 0.13.0 — corrections Text/Math

Base main 1b55bb025dc536a6df8dbbd8984438418fcf5245. Branche codex/0.13.0-math-selection.
Text/Math proportionnels par défaut (Shift libre), contrôles circulaires 6 px avec zone de prise inchangée,
cadre avec marge locale 6/4, géométrie du retournement calculée dans ce cadre. Images/crop inchangés.
bin(n,k), normalisation ∞ dans LaTeX, AltGr exclu des raccourcis Ctrl, Math GelPen et SVG utilisateur.
Presse-papiers WASM injecté en événements egui Copy/Cut/Paste, conserve curseur/sélection.
Les compilations Rust/WASM restent uniquement Actions. 187 tests Rust et le build WASM passent.
Les nouveaux scénarios Chromium binomial, proportions/Shift, copier/coller au curseur et AltGr passent
dans CI 37738814047 ; les clics UI attendent maintenant l'actualisation du survol/focus.
Le GPU logiciel CI peut perdre son instance et échouer à lire un PNG : la validation des pixels reste
distincte de celle des interactions. Le rendu local (binomial, GelPen, petites poignées et marges) a été observé.
La PR #14 centralise la fusion et la provenance du ZIP final DrafftInk_Windows_Portable_0.13.0.zip.
Pour reprendre, vérifier main/PR #14 et les workflows CI / Build DrafftInk Portable du commit fusionné.
Un aperçu isolé port 8897 reste ouvert ; préserver sa feuille et l'onglet utilisateur 8887.
Ne jamais utiliser les attentes download de Cua (ancien blocage) ; télécharger via le connecteur GitHub.

## Reprise 0.14.0 — draphtInQ / formes / saisie

Base main 9709a6062a13ed28aefdd86ad40e964108f7f4a4 (0.13 livrée, PR14).
Branche codex/0.14.0-branding-shapes, PR15. Travail local toujours autorisé,
compilation Rust/WASM exclusivement GitHub Actions. Validation en cours : ne pas annoncer fusion/livraison avant les workflows et la vérification du ZIP.

Nom d'application draphtInQ et favicon SVG noir transparent fourni par l'utilisateur.
Caret Text petit/décalé selon le mode Ctrl+haut/bas avant toute saisie ; glyphes
Math mesurés et dessinés avec la même police primaire, structures MATH conservées.
Catalogue de glyphes partagé ; pas de nouvelles allocations du catalogue par formule.
U+FFFC reste dans le document mais sa représentation de layout est U+200B (mêmes
offsets UTF8), supprimant le symbole OBJ. root(x,n) ouvre le mini éditeur, imbrication conservée.

Masquer les propriétés est activé par défaut dans Settings ; clic droit sur l'objet
ouvre les panneaux, clic extérieur les referme. Préférence inverse conserve les panneaux.
Menu clic droit Ellipse : ellipse/triangle/parallélogramme/trapèze/losange ; répétition
du raccourci configuré cycle les variantes (maintien ne cycle pas). Icône suit la forme.
Variantes géométriques dans Shape::Ellipse.geometry avec défaut rétrocompatible,
path/hit-test/miroirs locaux, mêmes styles, rotation, resize et Undo/Redo. Pas de collaboration.

Tests ajoutés : métriques police, caret, chemins/hit-test/JSON ancien et nouveau,
root imbriqué et interactions Chromium du menu et des panneaux. La suite historique
fixe explicitement hide_properties=false pour tester l'ancien affichage permanent.
Limite connue GPU logiciel : lire les résultats de capture séparément ; ne pas affirmer
une validation de pixels quand WebGPU perd son instance. Aucune mesure Edge RAM nouvelle.
Préserver le canvas ouvert port8897 et l'onglet8887 ; utiliser un aperçu isolé pour tester.


## Reprise 0.15.0 — Qraphtinc / Text / Math
Base main6c078b46c76b8e4820797bc04772f8583372f4e9, version0.14 livrée et CI/PortableOK. Branche codex/0.15.0-text-math-ux, PR16. Nom exact Qraphtinc. Stroke toujours visible sauf Ctrl+P ; seul Properties est contextuel. Plus de raccourcis symboles/formules dans Properties Text, commandes directes conservées. Mini panneau Text sans titre.
Text : Espace reste dans le script ; caret suit mode explicite et styles lors de navigation, dernière ligne vide corrigée, double clic mot câblé sur la sélection Parley. Ctrl+flèche = grand pas20, flèche et Shift =1pixel rendu, maintien/Undo conservés.
Math : fermeture/validation sur changement d'outil ou clic hors formulaire ; clic consommé et relâchement Idle ne crée pas de nouvel objet. Undo de la modification conservé. Police Math par défaut dans Settings, police par objet dans Properties, JSON rétrocompatible GelPen, restauration de polices locales et cache primaire. Aucun fichier Google privé copié.
Hitbox géométries sans remplissage : marge intérieure3 fois la tolérance historique ; marge extérieure inchangée. Rotation/miroirs/JSON/Undo conservés.
Rust/WASM via Actions uniquement. Tests modèles/métriques/caret passent sur ba1f403; interactions Chromium en validation. Ne pas annoncer fusion/livraison avant CI+Portable+ZIP. PR16 et outputs final permettront de retrouver provenance exacte.
Aperçus existants8897/tab7 et8898/tab8 à préserver. Un aperçu isolé8899 peut être en préparation (lire checkpoint15 local). Éviter absolument Cua.waitForEvent(download) ; utiliser connecteur GitHub. Poller le téléchargement shell jusqu'à exit_code0 avant d'utiliser le ZIP, un session_id n'est pas un téléchargement terminé.


## Reprise 0.16.0 — LaTeX natif, curseurs, bord et snapshots
Base main2cc3c72cf6e2bce7f0c8a48b3c475c0bb02ffa45 (0.15 livrée, CI/PortableOK), branche codex/0.16.0-latex-cursors-autopan. Utilisateur autorise la mise en oeuvre après explication, ignore « tt ». Math redevient un champ LaTeX brut : pas de conversion fractions/fonctions/scripts ni de sortie de bloc par espace/flèche. Texte garde les commandes intuitives. Les anciennes formules Math éditent leur LaTeX canonique sans supprimer leur source historique avant modification. Ctrl+haut/bas reste une insertion alternative de ^/_ ; pont touche morte conservé.
Auto-pan pendant déplacement d'objet (pas resize/édition) : zone48px, vitesse bornée600px/s, cadence16ms, caméra et position de l'objet synchronisées, arrêt sortie de bord ou relâchement ; Undo de drag conservé.
Snapshots Ctrl+S/automatiques horodatés localement yyyy-mm-dd_hhmmss nom ; collisions incrémentent seconde et vérifient fichiers existants. Autosave disque écrit PNG + JSON sous un même préfixe, récupération IndexedDB maintenue. Pas de demande de permission en arrière-plan.
Curseur Math SVG fourni, modesText/Math/standard contextuels : UI générique classique, formulaireMath spécial, Text édité+mini formulaire Text spéciaux même avec outilSelect. Resize/pan natifs gardés sur canvas. Bouton crayon de tab retiré, double clic conservé. Légendes Math/Text supprimées, exemple court selon la commande Text. AltGr Text ne devient plus raccourci Ctrl, permet {}.
JS helpers/dossier/snapshots passent localement. Tests Rust/parser et Chromium en préparation/validation Actions uniquement ; ne pas annoncer livré/fusionné avant CI+Portable+ZIP. Préserver la feuille utilisateur sur8899/tab9 (dernierWASM en mémoire peutêtre0.15avantdernierscorrectifs, fichiersstatiquesfinal0.15 ; pasrechargé). Anciennesfeuilles8897/8898 àpréserver si présentes. Aucun Rust build/test local ni police privée publiée, pas de serveur distant/collaboration.

### 0.17.0 — accent et sélection (en validation)

La 0.16.0 est livrée (PR #17 et correctif packaging #18, main 82e730609d5f485777a4131d78d15d44f4057ecd). Branche 0.17.0 : couleur d’accent persistante dans Settings et capture du geste de sélection commencé sur la feuille à travers les panneaux. Tests de persistance et interactions Chromium ; aucune compilation Rust locale. CI, fusion et ZIP ne sont pas encore validés ; consulter la PR de cette version pour leur statut final. wasm-pack reste explicitement en 0.15.0.

### 0.18.0 — Web hors connexion (en validation)

0.17.0 livrée : PR #19, main1645013d2a8a5c96c3b7c09fd54c758e1bc4a5ce, CI37797374127 huit contrôles verts/202 tests Rust, Portable37797374348 réussi et ZIP vérifié. Site utilisateur confirmé sur https://ilamartin.fr/qrapht/web/ (pas /qrapht/). Utilisateur demande explicitement une version avec cache hors connexion. Branche0.18 : service worker limité au sous-dossier, assets vérifiés par SHA256, cache versionné selon le build réel, activation de mise à jour sur clic, conservation du stockage utilisateur. Pas de Rust modifié ni compilation locale. Tests manifest et Chromium réel hors réseau/envoi partiel/mise à jour à valider en Actions ; CI, fusion et ZIP restent à confirmer dans la PR de cette version. FTP manuel utilisateur, aucune connexion serveur reçue.


### 0.19.0 — Qursor🌿, même ZIP local et FTP (en validation)

0.18.0 livrée : PR #20 fusionnée, main c97aa6c3d642007446dd57baa363dc045cb6ca1c, CI 37816053158 huit contrôles verts et 202 tests Rust, Portable 37816053165 réussi ; ZIP Web et Windows vérifiés, cache hors connexion testé. Branche codex/0.19.0-qursor-local-ftp : page renommée Qursor🌿, lanceur Lancer Qursor.cmd et guide unique Local/FTP. Artefact combiné contenant les mêmes fichiers web pour localhost et public_html/qrapht/web/. Clés du cache et données conservées ; tests des titres adaptés. Aucun code Rust modifié. Compilation via Actions uniquement. CI, fusion et livraison 0.19 restent à confirmer dans la PR ; pas d’accès FTP ni déploiement serveur effectué.

### 0.20.0 — Qurso🌿, SVG et édition (en validation)
0.19 livrée : PR21, main f1728cf0287b52bee467d3e81f41f24ca3297a7f, CI37821283349 et Portable37821283425 verts, ZIP Local/FTP vérifié. Branche codex/0.20.0-qurso-polish. SVG fournis utilisateur intégrés (qlogo/favicon/curseurs), logo central sur fond blanc et fondu, icônes pan/draw/erase remplacées et masques blancs pour teinte sélectionnée ; curseur Eraser dédié hors UI. Titres Properties et aide Forme retirés. Settings : labels lisibles au hover, Entrée sauvegarde et Échap annule les préférences. Nouveaux tracés sélectionnés et manipulables sans changer d’outil. Aperçu Math vide (plus x²). Gomme Manual étendue aux contours géométriques, remplissages conservés en fragments vectoriels fermés ; rotation, JSON et Undo/Redo couverts par tests. Après découpe, les formes deviennent fragments éditables (pas des ellipses/rectangles paramétriques).
Cache : sw.js public a renvoyé404 lors du diagnostic ; nouveau ZIP complet inclut les SVG dans le manifeste, erreur explicite et bouton Réessayer, install séquentielle pour éviter écritures tardives après rejet. Aucune tolérance aux fichiers corrompus/modifiés après build. Pas de FTP déployé. Les tests Rust et navigateur supplémentaires sont à exécuter via Actions ; ne pas annoncer fusion/livraison avant CI/Portable/ZIP vérifiés. Aucun Rust local ni fonte privée publiée.
PR22 fusionnée main90f4b658f49c0ff0a9ec83e57fd085b40568d176 après CI37833967146 huit contrôles verts et207tests. Livraison0.20 encore en attente : contrôle visuel Settings révèle les listes de polices sombres avec texte sombre ; branche codex/0.20.0-settings-contrast applique une palette claire cohérente au dialogue. Attendre cette CI et le nouveau packaging du main final avant ZIP.

### 0.21.0 — commandes de sélection et objets épinglés (livrée)
PR #24 fusionnée sur `main` (commit `d0fac2d8003d3751258c596f21ffdd341e584273`), CI de PR #24 `37879689032` réussi, workflow portable `37879689032` réussi et ZIP produit. Périmètre : Ctrl+flèche verticale, Ctrl+clic multi-sélection, écran de chargement jusqu'à la première frame, curseur Draw et icônes Select/Pan. Les objets épinglés restent à l'écran et conservent le fond et l'historique JSON/Undo/Redo. FTP non touché.

### 0.21.1 — épingles, curseurs et cache hors connexion (livrée)
PR #25 fusionnée sur `main` (commit `4e16cf11b5bafe4aff5e09b20602ffda4adffd0f`). Tous les contrôles CI de la PR passent ; workflow `Build DrafftInk Portable` #37896700997 réussi. ZIP portable vérifié (CRC, version, WASM/JS et ressources), SHA-256 `08b486b667283e0286bf96ab61b532c0abf7aa1a0c6fc7a1497cac320581f5d8`. Déplacement épinglé reconverti en pixels écran au relâchement ; poignées à taille constante ; commandes d'épingle et couleur de fond dans Properties ; Text finalisé avant épinglage ; marqueur centré au-dessus avec ombre ; curseurs SVG géométrie/gommes inclus au cache et au ZIP. Le diagnostic du cache affiche l'erreur détaillée du service worker avant le message générique. ZIP récupéré localement par l'agent ; aucun FTP touché.

### 0.22.0 — export des épingles, barre et raccourcis (en cours)
Branche `codex/0.22.0-pins-tools-settings` depuis main 0.21.1. Export PNG/clipboard/autosave inclut les objets épinglés à leur position de vue au moment de la capture, avec arrière-plan tourné selon l'objet. `+ Library` retiré des onglets, import bibliothèque déplacé sous Import Mermaid. Icônes Select/Pan cohérentes avec les états hover/actif standards, barre d'outils agrandie, icône Math épaissie. Settings permet de choisir les outils affichés dans la barre. Ctrl+flèche verticale traité avant le garde de focus egui. Draw normalisé après Highlighter. Scénarios de régression ajoutés pour export épinglé, déplacements verticaux et Highlighter→Draw. Compilation/test Rust/WASM exclusivement GitHub Actions ; ne pas annoncer livré avant CI et ZIP vérifié. FTP inchangé.
