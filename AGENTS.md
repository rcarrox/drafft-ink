# DrafftInk: instructions de travail

- Lire `docs/CLOUD_DEVELOPMENT.md` et `docs/CLOUD_STATE.md` avant de reprendre une version.
- Préférence cloud conservée pour la suite. Exception explicitement autorisée par l’utilisateur le 7 octobre 2026 : continuer en local pour le moment (autorisation persistante ; versions 0.8 et suivantes). GitHub Actions reste le compilateur ; ne pas supposer que cette tâche a été transférée dans le cloud.
- GitHub Actions est le compilateur Rust/WASM et fabrique les ZIP Windows. Ne demander aucune compilation Rust sur le PC utilisateur.
- Les tests utilisent les ressources du dépôt et `utils/generate_cloud_fixtures.py`. Ne jamais copier ou publier Google Sans Medium privée ; son chargement local fait partie de l'application finale.
- Préserver la saisie Math, Escape, pinch zoom/pan touchpad, JSON, Undo/Redo, polices et fonctionnement local Windows. Ne pas réintroduire collaboration, caméra, microphone, vidéo ou service applicatif distant.
- Travail de version : branche dédiée, PR, tests pertinents, CI, corrections des erreurs. Fusion/livraison selon l'autorisation de la demande actuelle. Vérifier le ZIP, sa version, ses JS/WASM et ses curseurs avec `utils/verify_portable.py`.
- Tests rapides : `node utils/test_browser_helpers.cjs`, `python3 utils/generate_cloud_fixtures.py --out work/fixtures`, vérification syntaxique des scripts. Rust/native/WASM via CI.
- Mettre à jour l'état de reprise dans GitHub après tout travail important. L'état doit distinguer ce qui est testé, non testé, fusionné et livré.
