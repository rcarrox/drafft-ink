# Qurso🌿 0.20.0 — site et cache hors connexion

Dans le ZIP Local et FTP, ouvrir le dossier `web` et envoyer **son contenu** (ou le contenu du ZIP Web séparé) dans `public_html/qrapht/web/`.
`index.html`, `offline.js`, `sw.js`, les SVG et le dossier `pkg` doivent être directement dans ce dossier.
Adresse : https://ilamartin.fr/qrapht/web/

Utiliser le **mode de transfert binaire** pour tous les fichiers. Envoyer `sw.js` **en dernier**, après tous les autres fichiers. Les fichiers du cache ont des empreintes : une version partiellement envoyée est refusée et le cache précédent est conservé. Ne pas modifier le HTML ou les fichiers compilés après fabrication du ZIP, ni activer une réécriture/minification automatique de leur contenu sur l’hébergement.

Ouvrir une première fois avec Internet et attendre **Disponible hors connexion**. La première mise en cache doit finir avant de couper le réseau. Ensuite, la même adresse fonctionne hors connexion sur ce navigateur/profil. HTTPS est requis sur le site. Le cache reste limité au dossier de l’application ; le reste du site ne change pas.

Une mise à jour affiche **Nouvelle version disponible** puis **Installer et recharger**. Enregistrer le travail avant de confirmer. La feuille n’est jamais rechargée automatiquement. Un autre onglet peut activer la version, mais la feuille courante reste ouverte ; la recharger après sauvegarde avant de reprendre le travail. Les préférences et documents locaux ne sont pas effacés par la mise à jour. L’application n’envoie aucun document au serveur.

Le cache du navigateur n’est pas une sauvegarde permanente : supprimer les données du site efface aussi ses données locales, et le navigateur peut libérer du stockage. Garder des exports JSON/PNG. Une première visite sans réseau ne peut pas fonctionner.

Les préférences du site HTTPS sont distinctes de celles du portable sur localhost. Les anciens documents du portable ne sont pas transférés automatiquement : utiliser Export/Import JSON. Pour Google Sans installée sur Windows, le site dépend de l’autorisation de polices locales du navigateur ; cliquer sur Actualiser mes polices dans Settings. Les polices privées ne sont pas dans ce ZIP et ne sont pas envoyées au serveur.

Le cache occupe du stockage sur disque. Il ne réduit pas à lui seul la RAM consommée par le rendu WASM/WebGPU.
