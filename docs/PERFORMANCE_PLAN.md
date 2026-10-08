# Mémoire — diagnostic et solutions proposées

Mise à jour 0.11 : plusieurs propositions sont maintenant implémentées. Lire MEMORY_0.11.md et la PR de codex/0.11.0-memory-budget pour les détails et la validation. Les propositions ci-dessous décrivent le point de départ 0.9 ; elles ne sont pas une mesure de RAM totale.

## Ce que signifient les 800 Mo

L’utilisateur indique 800 Mo pour l’ensemble d’Edge dans Windows, pas pour l’onglet DrafftInk seul. Ce total ne permet pas d’attribuer 800 Mo à l’application. Le point de départ est le gestionnaire du navigateur (Shift+Échap), avec les lignes onglet, GPU, navigateur et extensions, et la colonne mémoire JavaScript. [Documentation Microsoft](https://learn.microsoft.com/en-us/microsoft-edge/devtools/memory-problems/microsoft-edge-browser-task-manager).

Contexte : deux canvas, images 1200×2000. Une image RGBA décodée coûte 1200×2000×4 = 9 600 000 octets (9,6 Mo / 9,16 Mio), avant copie GPU, pools, historique et overhead du navigateur. La taille JPEG/PNG compressée ne représente pas la taille décodée.

Comparer, avec les mêmes onglets/extensions : Edge sans DrafftInk ; DrafftInk vide ; une image ; deux canvas ; vingt manipulations ; fermeture d’un canvas ; import/export. Relever la mémoire de l’onglet et du processus GPU, les allocations WASM et les octets des caches. La mémoire linéaire WASM grandit par pages ; un pic peut conserver une grande capacité même après libération des objets. [API WebAssembly Memory](https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/Memory/grow).

## Priorités de développement

| Priorité | Proposition | Intérêt / compromis |
|---|---|---|
| 1 | Instrumenter mémoire WASM, caches CPU, réserves/tampons GPU et import/export | Localise le poste réel ; évite une optimisation qui ne change que 20 Mo sur 800. |
| 2 | Ajuster les réserves du renderer GPU ; réutiliser les ressources d’export et éviter plusieurs textures pleine taille simultanées | Candidat prioritaire si une feuille vide consomme déjà beaucoup. Les ressources disponibles du moteur doivent être vérifiées avant de promettre une réduction. |
| 3 | Cache d’images décodées à budget et LRU, en donnant priorité au canvas actif ; par exemple un budget initial configurable de 64 Mio | Le code actuel retient les images de tous les canvas ouverts. Les données source restent intactes, les canvas inactifs sont redécodés à leur réouverture si nécessaire. |
| 4 | Aperçus d’image selon taille affichée/zoom, rendu des seules images visibles | Moins de pixels décodés/téléversés pour l’affichage ; original conservé pour export. Mise à jour progressive pour éviter un flou permanent à fort zoom. |
| 5 | Budget en octets pour Undo, en plus du nombre d’états ; diffs pour les grosses scènes | Les sources d’images sont déjà partagées depuis la 0.7. Les autres vecteurs/structures peuvent encore être clonés. Préserver un historique utile. |
| 6 | Réduire les copies pendant import/export et métadonnées ; dédupliquer les assets du JSON | Évite les pointes : base64, source encodée, pixels RGBA, texture, readback et PNG peuvent coexister. Déduplication nécessite un format compatible/versionné. |
| 7 | Séparer animation du caret/UI et rendu de la feuille ; invalider seulement les régions nécessaires | Améliore surtout CPU/fluidité. Les caches des formules et scènes existent déjà ; ne pas les reconstruire à chaque caractère si la formule n’a pas changé. |
| 8 | Mode économique à résolution temporaire de 75 % pendant pan/zoom | 56,25 % des pixels et environ 44 % de moins pour les tampons concernés, pas pour la RAM totale. Restaurer la netteté au repos et conserver l’export pleine résolution. |

Si le socle Vello/WebGPU reste trop lourd après ces mesures, prototyper un renderer Canvas 2D pour les scènes simples ou une véritable version native Windows. C’est un changement d’architecture à comparer sur les mêmes documents ; Electron conserve un socle Chromium et ne garantit pas une réduction.

## Ce qui n’est pas promis

Aucun profil complet des 800 Mo n’a été réalisé. Aucun objectif de RAM chiffré n’est annoncé. La 0.9 ajoute la saisie de commandes avec rendu vivant ; les optimisations ci-dessus restent des propositions, distinctes de cette livraison. Aucune compilation Rust locale n’est nécessaire : les prototypes/builds resteront sur Actions.
