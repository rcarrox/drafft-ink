# Qurso — principales évolutions

## 0.32.0

- Palettes entièrement interactives, y compris hors de Properties ; contrôles blancs et fond épinglé centré sur son panneau. Les changements de couleur restent annulables par geste.
- Settings devient une ligne du menu avec icône à droite ; infobulles gris clair et ombres rapprochées. Settings et Keyboard Shortcuts sont de vrais dialogues modaux. Un clic extérieur ferme le menu dès l’appui, sans dessiner derrière.
- Raccourcis complets et modifiables dans Keyboard Shortcuts, sauvegardés et exportés ; conflits signalés. Ctrl+Shift+R réinitialise la disposition des panneaux. Config Reset rétablit les réglages de l’application sans effacer les tableaux/presets.
- Time : options fermées immédiatement par la croix, choix de police fermé après sélection. Time propose Monospace, Inter, Noto Sans et GelPen ; chiffres en colonnes fixes, centrage stable, curseur de resize sur toute la zone des poignées. Inter 4.1 embarquée sous SIL OFL.
- Formules dans Text : caret projeté dans le rendu à partir du code, fractions imbriquées incluses ; bloc actif légèrement surligné, clic dans la formule pour positionner le caret. Symboles π/moins et fallback suivent la police du texte ; séparateurs LaTeX ajoutés après les symboles.
- Optimisations : réutilisation de la texture du tableau pour les animations d’interface et le clignotement du caret ; aperçus des images épinglées indépendants du zoom caméra, plafonnés aux pixels source et réutilisés aux zooms proches. Horloges rafraîchies selon les chiffres visibles, formateurs de date réutilisés.
- Images originales conservées pour export ; caches bornés, données Undo partagées et ressources des canvas inactifs conservées selon les contrôles existants. Rapport de mesures généré en CI, sans assimiler le navigateur entier à la mémoire de Qurso.

## 0.31.0

- Import de plusieurs presets PNG en une fois ; noms indépendants des fichiers et taille source conservée.
- Presets prédéfinis du dossier web/presets importés au premier lancement, sans restaurer ceux supprimés ensuite.
- Menus et champs PNG Preset sur fond blanc.
- Ctrl+clic retire un objet d’une sélection multiple, y compris sur une poignée de forme ; Ctrl-drag pour rogner une image est conservé.
- Export/import des paramètres personnalisés dans un JSON portable : couleurs, polices, raccourcis, disposition et autres options. Les autorisations de dossier et les polices privées restent propres à l’appareil.
- Menu Insérer : Image, Time, Preset et Numworks (nouvel onglet vers numworks/). Import PDF retiré.
- Fonctions images, presets et Time chargées indépendamment du cache hors connexion.

## 0.30.0

- Sélecteurs de couleur harmonisés, couleurs Stroke personnalisables et persistantes.
- Settings organisé en Apparence/Data, version affichée.
- Options Time sur fond blanc avec fermeture par croix/Échap.
- Fonctionnement en ligne sans installation automatique de cache, récupération IndexedDB conservée.
