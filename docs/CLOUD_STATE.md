# État de reprise — 0.11.0

7 octobre 2026. Travail local autorisé ; Rust/WASM compilés exclusivement par GitHub Actions.
Branche codex/0.11.0-memory-budget. Lire la PR de cette branche pour les statuts CI/fusion/artefacts finaux.

Demande : alléger l’application, cible comparable aux 200 Mo de Paint.NET. Les 800 Mo précédemment relevés étaient l’ensemble d’Edge dans Windows ; ne pas attribuer ce total à l’onglet et ne pas annoncer un total inférieur à 200 Mo sans mesure comparable.

Changements en validation : réutilisation texture d’écran/scène ; moteur GPU Area-only et partagé à l’export ; readback libéré avant encodage PNG ; cache LRU images 32 Mio avec aperçus zoom/2048 px, export original complet sans rétention de son décodage ; caches des canvas inactifs libérés ; documents/historiques déplacés entre canvas sans copie ; historique de payloads clonés 16 Mio et cache de géométrie environ 4 Mio ; WASM chargé par streaming ; diagnostics de composants et de mémoire WASM/JS. Les budgets de cache ne sont pas la RAM totale. Le moteur Vello conserve des réserves GPU fixes importantes ; ne pas les réduire aveuglément sans protection contre les dépassements.

Tests ciblés natifs et Chromium : aperçus/qualité originale/JSON source inchangée, LRU et suppression des caches, historique Undo/Redo, absence d’accumulation des chemins pendant drag, réutilisation texture et basculement retour entre deux canvas. Maintenir les régressions 0.10 et le pinch/pan.

Dernière livraison : 0.10.0, main04b84a6534f8e020b6571c08dd165ba6347e9d09, PR #11 fusionnée, CI37660570262 et portable37660570353 réussis. ZIP SHA256c3d8dba54a211d35d2a5cef561d54821d5322dac714d80e37837e3b8b6aee662. Publication exacte dans la description de PR11.

Les vérifications visuelles restent locales ; le WebGPU logiciel du CI peut perdre son instance et les captures uniformes sont signalées comme indisponibles, jamais comme succès visuel. Les événements Beeftext sont simulés. Clippy historique non bloquant. Aucun environnement Codex Cloud publié par cette tâche. Aucune police privée publiée.
