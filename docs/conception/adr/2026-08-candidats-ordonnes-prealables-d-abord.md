# Les candidats sont ordonnés préalables d'abord, agrégats en dernier

## Contexte

L'ordre des variables de la recherche B suivait l'ordre d'entrée (alphabétique via l'intake).
Sur le B-GMC, les préalables MAT-11xx trient après tous les codes GMC : chaque descente plaçait les dépendants tôt, échouait au tour du préalable, et le DFS re-permutait la queue — la première feuille relâchée restait hors d'atteinte d'un budget de 20 millions de nœuds (diagnostic du 2026-08-19, B-GMC à 0/120 au premier contact).
Symétriquement, un cours à « Crédits exigés » affecté tôt s'épingle à l'aveugle : il ne peut juger sa session qu'une fois tout le reste assis (PHI-2910 et PHI-3900 rejetaient chacun des millions de feuilles).

## Décision

`build_candidates` trie les candidats par clé `(porte-un-seuil-de-crédits, profondeur-de-préalables)`, stable sur l'ordre d'entrée :

- profondeur = longueur des chaînes de préalables *listés dans l'ensemble* (relaxation bornée à n passes — un cycle plafonne au lieu de boucler ; l'auto-mention est ignorée) — les préalables s'affectent avant leurs dépendants, chaque descente reste constructive ;
- les cours à seuil de crédits passent en dernier, quelle que soit leur chaîne.

L'ensemble des solutions ne change pas (l'ordre des variables ne change que l'ordre d'énumération) ; les fixtures gelées comparent des ensembles triés et ne bougent pas.

## Alternatives rejetées

- **Semer depuis les cheminements types manuels** : ne couvre que les programmes qui en ont un, et ne corrige pas la recherche elle-même.
- **Heuristique dynamique (most-constrained-first)** : recalcul à chaque nœud pour un gain que l'ordre statique suffit à obtenir ici.
