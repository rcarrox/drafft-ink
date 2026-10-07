# État de reprise — 0.8.0

Date : 7 octobre 2026.

- Dépôt : `rcarrox/drafft-ink`, application locale Windows/Rust/WASM.
- Travail 0.8 : branche `codex/0.8.0-text-images-presentation`, [PR #9](https://github.com/rcarrox/drafft-ink/pull/9).
- Le 7 octobre, l’utilisateur a explicitement demandé de continuer en local pour le moment. Cette exception remplace l’exigence cloud pour ce travail ; GitHub Actions reste le compilateur Rust/WASM.
- **Publication exacte** : lire l’état de fusion et la description de PR #9. Sa description de livraison contient le commit `main`, le dernier CI, le workflow portable et l’identifiant/hash du ZIP effectivement vérifié. Ne pas assimiler un build de branche à l’artefact final.

## Code et ressources

Text conserve le caret littéral, y compris une touche morte française, les séquences `^^`, `^p`, `^4`, `^>` et `^<`, ainsi que les lettres accentuées. Les suppressions d’un expander correspondent désormais aux caractères réellement présents. Ctrl+haut/bas reste l’alternative exposant/indice. La sélection dispose de Ctrl+B/I/U ; spans, couleurs et positions des formules suivent les remplacements Unicode et sont conservés en JSON.

Text contient des blocs de formule dans son flux : fractions imbriquées, racines carrées/n-ièmes, sommes, produits, intégrales et limites, avec champs séparés et saisie conviviale. Les anciens objets Math restent pris en charge. La police math embarquée est réellement **STIX Two Math** malgré son ancien nom de fichier `rex-xits.otf` ; sa vraie famille est utilisée en secours pour ≥, ≤, ∏, ∫, etc. Les Google Sans privées restent uniquement sur le PC utilisateur. Le texte utilise une police embarquée en attendant une police locale manquante.

Les images ont des drapeaux miroir indépendants. Traverser le bord opposé avec une poignée retourne le contenu sans altérer la source. Rotation près des coins : angle relatif au point de pression, pivot au centre, Shift pour le snap. Le crop tient compte du miroir et de la rotation. JSON et Undo/Redo conservent les transformations.

Ctrl+P cache les panneaux/outils/onglets. F11 utilise le plein écran Chrome/Edge. Le cache Text conserve une scène complète avec styles/formules et coordonnées de variation de fontes. Aucun service applicatif distant, caméra, micro ou collaboration n’est ajouté.

## Preuves et limites

La révision `022c43c95a3527d06bc4e36fc893dfa3332ad1bf` a passé les huit jobs du [CI #46](https://github.com/rcarrox/drafft-ink/actions/runs/37578399416), dont Rust, WASM, natifs Ubuntu/macOS et Chromium Interactions. Le dernier head de PR #9 doit passer à nouveau avant fusion. Clippy historique reste non bloquant ; consulter ses logs plutôt que déclarer le lint entièrement propre.

`utils/test_browser_interactions.cjs` pilote le vrai build WASM, avec touches ordinaires, paquets Unicode et touche morte simulée. Il vérifie `123^4` → `123⁴`, ≥/≤, doubles carets/accents, mise en forme partielle, fraction imbriquée/racine n-ième, présentation/plein écran, miroir/rotation d’image 1200×2000 et Undo/Redo. Les captures, états et logs sont dans l’artefact `chromium-interaction-evidence`. Le diagnostic `?drafftink-test=1` est uniquement en lecture.

La simulation n’est pas un essai physique du programme Beeftext sous Windows. Les racines/fractions/sommes/produits/intégrales/limites et glyphes sont aussi testés côté Rust. Les captures doivent être inspectées ; une simple présence de PNG n’est pas une validation visuelle. La mémoire Edge à 800 Mo n’a pas été mesurée.

## Reprise cloud

Aucun environnement Codex Cloud n’a été créé/publié par cette tâche. Les scripts et ressources du dépôt restent portables ; lire `CLOUD_DEVELOPMENT.md` lorsque le passage cloud est effectivement décidé. La suite Chromium demandée pour la préparation est maintenant implémentée en Actions. Ne pas publier les polices privées ni dépendre des chemins Windows de développement.

Dernière livraison antérieure : 0.7.0, commit `9b16b66ddf8cc545ebfe479240ea3f8acefb4b87`, portable Actions `37550413580`, artefact `11452063008`, SHA256 ZIP `9b8d9bd27ccb6e084550efaf09cce9822e14592a07eaa047d1c33ac84e817a16`.
