# Allègement mémoire — 0.11.0

## Objectif et comparaison

L’objectif utilisateur est un outil comparable à Paint.NET, annoncé vers 200 Mo. Les 800 Mo signalés précédemment représentaient l’ensemble d’Edge. Pour comparer, relever l’onglet DrafftInk et la différence navigateur/GPU entre Edge vide et le même document, avec les mêmes extensions, dimensions et zoom. Aucun total réel inférieur à 200 Mo n’est confirmé à ce stade.

## Modifications

| Composant | Avant | Après |
|---|---|---|
| Texture d’écran RGBA | Nouvelle allocation à chaque rafraîchissement | Une allocation réutilisée, remplacée seulement si la fenêtre change de dimensions |
| Scène CPU | Buffers abandonnés après le rendu | Buffers recyclés et contenu remis à zéro |
| Pipelines GPU | Area + MSAA8 + MSAA16 préparés | Area seul, qui était déjà l’anticrénelage réellement utilisé |
| Export PNG WASM | Second moteur GPU et scratch buffers | Moteur existant partagé, même ordre de soumission GPU ; readback détruit après copie, avant encodage |
| Images | Original complet décodé et retenu pour les canvas ouverts | Aperçus selon zoom, grand côté 2048 px maximum ; LRU décodé 32 Mio ; caches inactifs libérés |
| Qualité/source/export | Sources conservées | Sources toujours inchangées ; export construit sur les originaux complets, sans polluer le cache d’aperçus |
| Canvas/historique | Document et historique clonés à l’entrée/sortie de canvas | Propriété déplacée ; un seul document vivant par canvas |
| Undo/Redo | 50 snapshots, sans limite en octets | 50 maximum et environ 16 Mio de payloads clonés par canvas ; dernier état de chaque pile conservé même si trop grand |
| Géométrie de traits | Jusqu’à 500 variantes, y compris anciennes géométries | Une géométrie actuelle par objet/trait ; suppression des objets disparus et budget de payload d’environ 4 Mio |
| Chargement WASM | Chunks + copie complète en JavaScript | Instanciation en streaming avec la réponse réseau |

L’affichage réduit des sources de plus de 2048 pixels peut être moins détaillé à fort zoom. Le document, le crop et les pixels originaux exportés restent inchangés. Les caches libérés sont reconstruits au retour au canvas. La première réouverture d’une image peut donc nécessiter un nouveau décodage.

## Mesures et limites

Cas natif ciblé : source 1200×2000, objet affiché 160×120 à zoom1. L’ancien cache décodé représentait 9 600 000 octets ; l’aperçu attendu 153×256 représente 156 672 octets, soit 98,37 % de moins pour ce cache précis. Ce n’est pas une baisse de 98,37 % de toute la RAM. Le test vérifie aussi l’original 1200×2000 à l’export et la conservation de la source. Résultats effectifs à lire dans le CI de la PR.

Le runner Chromium écrit memory.json avec les octets du cache, de l’historique, du rendu écran et la capacité de mémoire WASM. Le diagnostic drafftinkMemoryUsage rapporte WASM et heap JavaScript ; il ne mesure ni toute la RAM d’Edge ni la VRAM. Ces valeurs peuvent recouvrir des postes comptés par le navigateur et ne doivent pas être additionnées comme une mesure totale. La capacité WASM peut rester haute après une allocation temporaire, même si le cache a été libéré.

Le moteur Vello épinglé dans Cargo.lock réserve encore de gros buffers GPU pour ses scènes générales (voir son vello_encoding/src/config.rs à la révision1771112ebc0ae04d996ebf493d3a00d73303f014). Ils ne sont pas tous exposés à l’application. Un vrai budget global garanti pourrait nécessiter un moteur plus léger ou une allocation adaptative avec contrôle des dépassements et reprise correcte ; diminuer simplement les constantes risquerait de perdre des traits.

Les originaux encodés, les scènes visibles, le décodage temporaire et l’export ne sont pas tous inclus dans les budgets de cache. Les très grands documents peuvent dépasser ces budgets de composants. Le seuil de 200 Mo doit être vérifié sur l’usage réel ; aucun profil Windows Edge avant/après n’est encore réalisé.
