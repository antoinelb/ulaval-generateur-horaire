# L'étoile de concomitance survit au parsing, honorée par feuille

## Contexte

Le répertoire accole un `*` au sigle d'un préalable qui « peut être suivi simultanément » — la page GEX-3001 le glose elle-même sous la liste.
Le tokeniseur des préalables le supprimait purement et simplement (`raw.replace("*", "")`) : l'arbre de GEX-3001 rendait `"GCI-2010"`, indiscernable d'un préalable strict.
Le solveur n'avait alors qu'un réglage global, tout ou rien, pour rendre la concomitance possible — et un relevé réel qui suit une paire étoilée en concomitance se faisait refuser son propre passé (ADR `2026-08-concomitance-ouverte-par-le-releve`, mesure provisoire).
400 des 8834 cours du catalogue portent au moins une étoile.

## Décision

`PrereqTree` gagne une feuille `Concomitant { concomitant: String }`, sérialisée `{"concomitant": "GCI-2010"}` **à côté** de la feuille chaîne, qui garde son sens strict — la sérialisation reste donc rétrocompatible pour le dépôt JS `grille-de-cheminement-interactive`, qui n'affiche que `prerequisites.raw`.
Le tokeniseur ne retire plus le `*` : c'est `checkable_operand` qui reconnaît `SIG-0000*` et émet la feuille étiquetée.
Une étoile sur un opérande hors grammaire n'est pas retirée non plus — MAT-2910 garde `{"raw": "IFT 10426*"}` : rien d'étoilé n'est perdu ni avalé.

Côté solveur, `FlatNode::Course` porte le drapeau et `course_leaf` lit `placé < session || (étoile && placé == session && ≠ soi)`.
Le réglage global `concomitant` n'est plus qu'une **dérogation** : il accorde à toute feuille ce que l'étoile accorde déjà à la sienne.
Les fonctions statiques `prerequisites_met` / `unmet_prerequisites` reçoivent l'ensemble « même session » **en plus** de l'acquis strict, pour que la question par rangée et le diagnostic d'épinglage lisent l'étoile comme le solveur ; côté UI, un préalable posé la même session sans étoile est nommé comme tel, distinct d'un préalable absent.
L'écho du champ de correction lit l'étoile à voix haute (« GCI-2010 peut être suivi la même session (concomitance permise) ») : le symbole n'est jamais laissé à deviner.

Les arbres de `data/cours.json` sont re-dérivés de leur propre `raw` par la même fonction `parse_prereq_tree` que le scraper appelle — 400 arbres réécrits, zéro requête, le re-scrape restant au cron habituel.

## Alternatives rejetées

- **Changer `Course(String)` en `Course { code, concomitant }`** : casse la sérialisation que le dépôt JS lit, et réécrit une quarantaine de constructions de tests pour un drapeau presque toujours faux.
- **Garder l'étoile dans le texte de la feuille** (`"GCI-2010*"` comme sigle) : chaque consommateur devrait la retirer avant de comparer un code — une bombe à retardement à chaque `contains`.
- **Ne relâcher que le réglage global** (le statu quo) : l'étudiant porte une décision que la donnée connaît déjà, et l'applique à des préalables que le répertoire, lui, exige strictement.
- **Re-scraper pour régénérer les données** : ~10 000 requêtes pour une information que le `raw` déjà stocké porte intégralement.
