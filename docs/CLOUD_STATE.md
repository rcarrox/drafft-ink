# État de reprise

Date : 7 octobre 2026.

- Dépôt : `rcarrox/drafft-ink`, application locale Windows/Rust/WASM.
- Dernière version fonctionnelle : **0.7.0**, fusionnée par PR #7.
- Commit applicatif : `9b16b66ddf8cc545ebfe479240ea3f8acefb4b87`.
- CI de branche : `https://github.com/rcarrox/drafft-ink/actions/runs/37549241865` — jobs réussis, 160 tests Rust et 6 contrôles JS.
- CI de main : `https://github.com/rcarrox/drafft-ink/actions/runs/37550413536` — vérifier son état courant avant reprise.
- Portable : `https://github.com/rcarrox/drafft-ink/actions/runs/37550413580` — réussi ; artefact `DrafftInk-Windows-Portable-0.7.0`, ID `11452063008`.

## Réalisé

Police retenue pour les futurs textes et configurable dans Settings ; Google Sans Medium chargée localement via le lanceur Windows et utilisée en Math ; face de police conservée dans le JSON. Panneaux déplaçables, disposition couleurs/traits sur deux colonnes, palette Laser au clic droit, SVG souris/Text/Math, couleur du contour configurable, huit poignées de resize avec axes locaux et crop des bords. Source images partagée dans Undo/Redo, cache par source pour les documents ouverts, math/cmap en cache et caret à réveils espacés.

## Validation et limites

Le registre Windows local a fourni 622 entrées, dont Google Sans Medium ; la route de fonte a renvoyé un TTF valide et une route non enregistrée a été rejetée. Contrôles visuels locaux : Google Sans Medium/poids Medium, texte caractère par caractère, `x^3`/caret, panneaux compacts, palette Laser. Les tests sont dans le dépôt ; la police privée ne l'est pas.

Le moteur Winit ne doit pas être testé uniquement par une injection DOM globale `insertText` : pendant le contrôle local, l'outil d'injection globale d'un mot n'a produit que son premier caractère alors que les événements de touches séparés produisaient le mot correctement. Les tests navigateur de reprise doivent traiter cela comme un cas à diagnostiquer, sans attribuer automatiquement le défaut au clavier physique.

La consommation de 800 Mo dans Edge n'a pas été profilée. Contexte utilisateur : deux canvas et image(s) de 1200×2000. Réduire les copies/caches évitables est réalisé ; mesurer WASM, textures GPU et overhead navigateur reste requis pour chiffrer les gains. Propositions : niveaux de détail d'images, séparation de l'affichage UI/feuille, budget d'historique/caches et stockage d'assets dédupliqué.

## Suite cloud

La branche de préparation `codex/cloud-ready-0.8` ajoute instructions, bootstrap, serveur et fixtures portables, vérification des ZIP. Ce document sera actualisé après les checks de cette préparation. **Aucun environnement Codex Cloud n'a encore été créé ou publié par cette tâche.** Suivre l'unique configuration initiale décrite dans `CLOUD_DEVELOPMENT.md`, puis lancer les travaux 0.8 depuis l'environnement cloud.

Avant la première modification visuelle 0.8, mettre en place et valider le runner Chromium headless/CI décrit dans le guide. Ce point est une préparation planifiée, pas une suite E2E déjà réussie. Ne pas attendre le retour du PC Windows pour les tests unitaires, la compilation ou le packaging.

ZIP 0.7.0 vérifié : CRC, version, lanceurs, JS/WASM et SVG présents ; aucun fichier Google Sans privé inclus. SHA256 du ZIP portable intérieur : `9b8d9bd27ccb6e084550efaf09cce9822e14592a07eaa047d1c33ac84e817a16`. Le fichier WASM généré pèse 9 923 927 octets. CI main 0.7.0 terminé avec succès.
