# Développement cloud à partir de la 0.8

Le développement, les tests et la fabrication des versions peuvent se faire à distance. L'application finale reste un ZIP exécuté localement dans Chrome/Edge sous Windows. Lire une police installée chez l'utilisateur est une fonction du lanceur local, pas une dépendance du développement.

## Une configuration initiale dans l'interface

Dans Codex, choisir **Work in > Cloud > Select environment > Create environment**, sélectionner `rcarrox/drafft-ink` et connecter GitHub si demandé. Fournir le bloc de configuration ci-dessous pendant la préparation. Après vérification du rapport, sélectionner **Publish**, puis démarrer une nouvelle tâche avec cet environnement.

Cette création/publication nécessite l'interface du compte ; elle n'a pas été réalisée par la tâche locale actuelle. La tâche Windows existante n'a pas été transférée. Les versions suivantes doivent démarrer dans le nouvel environnement publié.

[Documentation officielle : Cloud environments](https://learn.chatgpt.com/docs/environments/cloud-environments), consultée le 7 octobre 2026. Les outils interactifs de contrôle navigateur/ordinateur ne sont pas actuellement disponibles dans l'environnement cloud ; les contrôles de navigateur doivent donc devenir des tests programmatiques exécutés en CI, avec captures comme artefacts.

## Configuration à fournir

```text
Préparer un environnement de développement pour rcarrox/drafft-ink depuis main.
Utiliser Node.js 22, Python 3, Git et rustfmt correspondant à Rust 1.91.1.
Exécuter bash scripts/cloud/setup.sh, puis bash scripts/cloud/check.sh.
Les compilations et tests Rust/WASM, ainsi que la fabrication du ZIP, restent sur GitHub Actions.
Permettre l'accès GitHub nécessaire au dépôt et aux logs/artefacts Actions.
Ne pas dépendre d'un ordinateur Windows connecté, de polices privées ou de fichiers hors dépôt.
Pour une prévisualisation technique, démarrer python3 scripts/cloud/preview.py --port 8888.
Ne pas installer un client/exécuteur sur le PC utilisateur pour ce travail.
Publier l'environnement seulement après vérification des commandes et du rapport.
```

Le script d'installation peut être enregistré comme **Install script**. Les instructions de démarrage peuvent être enregistrées comme **Start skill** dans l'environnement. Aucun fichier de configuration prétendant créer automatiquement un environnement Codex n'est inventé dans le dépôt.

## Procédure d'une version

1. Lire `AGENTS.md` et l'état de reprise, vérifier le `main` réellement fusionné et créer une branche dédiée.
2. Implémenter et ajouter des tests pertinents. Les données de test doivent être générées depuis le dépôt.
3. Ouvrir une PR et suivre CI : Browser Helpers, Test, Build WASM, builds natifs et Dependency Age Gate. Clippy est historiquement non bloquant ; ne pas confondre cette tolérance avec l'absence de problèmes de lint.
4. Diagnostiquer les logs et corriger. Ne pas demander au développeur de lancer Rust sur son PC.
5. Après les checks et dans le périmètre autorisé, fusionner. Attendre `Build DrafftInk Portable` sur le commit de `main` concerné.
6. Télécharger l'artefact exact, extraire le ZIP portable intérieur et exécuter `python3 utils/verify_portable.py ZIP --version VERSION`.
7. Fournir le ZIP, les liens CI/PR, un résumé et les limites des mesures. Mettre à jour `docs/CLOUD_STATE.md`.

## Ressources et tests portables

- `web/cursormouse.svg` et `web/cursortext.svg` sont les ressources réellement utilisées ; aucun téléchargement depuis le bureau du développeur n'est requis.
- `utils/test_browser_helpers.cjs` teste les SVG, couleurs, exceptions de curseur resize, réveil de rendu, catalogue Windows simulé et cache local des polices.
- Les tests Rust couvrent cursor Math, touche morte/composition, Escape, fonts/defaults, snapshots partagés, resize/crop tournés, poignées de bord, JSON et caches.
- `utils/generate_cloud_fixtures.py` fabrique une image déterministe de 1200×2000 et un catalogue de test. Sa police est le Noto Sans public déjà inclus dans le dépôt, jamais Google Sans privée.
- `tests/fixtures/keyboard-fr.json` décrit les gestes et sorties attendues pour un futur runner clavier/navigateur. Ce fichier n'est pas une preuve d'un test physique sur un clavier français.
- `scripts/cloud/preview.py` sert le build Actions et les seuls fixtures de police publics. L'application cloud de test peut exercer le chargement d'une police sans accéder au registre ou aux fontes privées de Windows.

## Contrôles d'interaction à automatiser avant une évolution visuelle 0.8

Un runner Chromium/Playwright en GitHub Actions doit télécharger l'artefact WASM de la même révision, démarrer le serveur de fixture, fixer la fenêtre à 1280×720 et utiliser de vrais événements `keydown/keyup` caractère par caractère (pas une injection de texte globale seulement). Vérifier la saisie `x^3`, `x_1`, Ctrl+haut/bas, sortie d'un bloc, Escape, maintien de la police, déplacement des panneaux, clic droit Laser, poignées/crop et Undo/Redo. Joindre captures et logs à la CI. Si le GPU logiciel n'est pas disponible, signaler cette limite : un screenshot n'est pas validé par une simple présence de fichier.

Les validations 0.7 citées dans l'état incluent des contrôles manuels locaux. Elles ne doivent pas être rebaptisées contrôles cloud. Les gains de mémoire Edge restent à mesurer ; le développement cloud ne nécessite pas l'accès à cette machine pour avancer sur les optimisations structurelles.
