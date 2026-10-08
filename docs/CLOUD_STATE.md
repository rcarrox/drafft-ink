# Reprise — 0.12.0

8 octobre 2026. Branche codex/0.12.0-text-save-controls ; lire sa PR pour validation/fusion/artefact exacts. Travail local autorisé, Rust/WASM exclusivement Actions.

Demandes : nudge fin 1 pixel vs Shift pas20 historique et répétition ; vrais styles exposant/indice sur tout caractère de Text ; font parent appliquée aux chiffres/lettres des formules inline ; Ctrl+S PNG complet et autoPNG périodique dans dossier autorisé ; vitesse pinch2× configurable ; Stroke plus compact. EXE uniquement une question : ne pas construire/remplacer Chrome/Edge sans demande. Une version native pourrait réduire le coût navigateur mais doit conserver fonts/touchpad et être mesurée ; wrapperElectron conserveChromium.

En validation. SourceAuto précédente écrivait uniquementIndexedDB ; garder récupération maisajouterPNG.Ctrl+Shift+S JSON. Autosave sanspermission ne doit pas télécharger ni demanderpermission enarrière-plan. Statutvisible,récupérationaprèséchec,unchangedskip. Les caractères restentinchangés.JSONscriptdefault0compatible. Préservermémoire0.11,pinchpan,Beeftext,cropmirrorUndo et math. Pas deprivatefonts publiées. TestsGPUlogicielCIlimités,ne pasaffirmerpixelsvalides surcaptureuniforme.

Dernière livraison0.11 : mainb7c6440e1bedee6e90e77f34aac4dd061eb3ff6e, PR12, CI37715815143/Portable37715815060 réussis. ZipSHA2565d42480cd9c552d0df647409aac5bcfbfbdc0e6255148ec55827678de0556233. 200MoEdgeTotalnonvérifié ; WASM21%de moins cas2images.

IMPORTANTOUTILS : ne jamais réessayer Cua.waitForEvent(download) ; a bloqué20958secondes. TéléchargerlesartifactsviaGitHub et testerfilesystemavecVM/mocks/CI plutôtqu’attendreunévénementdedownloadducontrôleurnavigateur. Usertab8887 àpréserver.
