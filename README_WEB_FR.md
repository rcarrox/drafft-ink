# Qurso🌿 0.32.0 — site en ligne

Dans le ZIP Web, envoyer le contenu du dossier `web` vers `public_html/qrapht/web/` en mode binaire. `index.html` et le dossier `pkg` doivent se trouver directement à cet emplacement.

Adresse : https://ilamartin.fr/qrapht/web/

Qurso fonctionne en ligne. Il ne nécessite pas de service worker ni de fichier `sw.js`. Les documents et préférences restent enregistrés dans le stockage local du navigateur ; IndexedDB permet de retrouver les tableaux après fermeture de l’onglet. Garder également des exports JSON/PNG pour les sauvegardes.

Les préférences du site HTTPS sont distinctes de celles de la version Windows sur localhost. Pour Google Sans installée sur Windows, autoriser l’accès aux polices locales dans le navigateur puis cliquer sur « Actualiser mes polices » dans Settings. Les polices privées restent sur votre PC.

Presets prédéfinis : déposer les PNG dans `presets/`, actualiser `manifest.json` avec `Actualiser presets.cmd`, puis envoyer le dossier. Numworks : déposer votre simulateur dans `numworks/` (index.html requis). Les paramètres se transfèrent via Settings → Data → Exporter/Importer les paramètres. Voir EVOLUTIONS.md.
