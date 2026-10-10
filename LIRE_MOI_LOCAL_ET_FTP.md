# Qurso🌿 0.31.0 — un ZIP pour Windows et votre FTP

## En local sur Windows

Décompresser **tout le ZIP**, puis double-cliquer sur **Lancer Qurso.cmd** et choisir Chrome ou Edge. Conserver le dossier `web` et les autres fichiers à côté du lanceur. Aucun compilateur ni installation nécessaire. Utiliser le lanceur plutôt que double-cliquer sur `index.html`.

## Sur votre FTP

Envoyer **le contenu du dossier `web`** dans **public_html/qrapht/web/** en mode binaire. Le fichier `index.html` et le dossier `pkg` doivent être directement dans ce dossier. Les lanceurs Windows restent sur votre PC.

Ouvrir **https://ilamartin.fr/qrapht/web/**. Qurso fonctionne en ligne et n’installe pas de cache hors connexion. Les tableaux restent récupérables dans IndexedDB après la fermeture de l’onglet ; conserver aussi des exports JSON/PNG.

Les deux modes utilisent exactement les mêmes fichiers. Leurs documents et préférences sont stockés séparément par le navigateur : utiliser Export/Import JSON pour transférer une feuille entre local et site.

Voir aussi `README_LOCAL_FR.md` et `README_WEB_FR.md`.
