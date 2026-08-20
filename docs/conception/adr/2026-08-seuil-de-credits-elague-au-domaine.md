# Les « Crédits exigés » élaguent au domaine et se propagent en cours de recherche

## Contexte

Un seuil `program_credits` n'était jugé qu'en feuille (`finalize`) : trois valuations le laissaient `Unknown` tant que l'affectation n'était pas complète.
Sur le B-GMC, PHI-2910 (30 cr) puis PHI-3900 (60 cr) rejetaient chacun des millions de feuilles pendant que le DFS permutait tout ce qui suivait — aucune feuille relâchée dans un budget de 20 M de nœuds, grille vide au premier contact.

## Décision

Trois couches, toutes des bornes supérieures prouvées (la concomitance ne relâche jamais `program_credits`, « strictement avant » tient partout) :

1. **Capacité de préfixe** (`prefix_capacity`) : le maximum de crédits que les sessions 1..s peuvent contenir — plafond partout où un cours régulier entre ; un été fermé ne compte que ses stages *dont le propre seuil est atteignable à ce point* et ce qui y est épinglé, borné au plafond.
2. **Élagage au domaine** : une session dont le plafond de préfixe ne peut pas atteindre le seuil (`tree_admits_ceiling`, optimiste partout ailleurs) sort du domaine avant la recherche. Le pré-écran nomme d'abord l'arbre insatisfiable (diagnostic plus précis qu'un domaine vide, l'élagage pouvant vider le domaine).
3. **Propagation** (`credit_watch` + borne de potentiel) : les arbres à seuil sont re-vérifiés à **chaque** extension (ils dépendent de toute l'affectation, `referenced_by` ne les rappelle jamais) ; `credits_leaf` répond `False` dès que `avant + non-affectés-pouvant-encore-précéder < seuil` — le potentiel ne fait que décroître le long d'une descente, l'élagage est définitif. Un cours surveillé **laissé de côté** est sauté (rien n'est exigé pour ne pas suivre un cours — le miroir du saut de `finalize`, sans lequel la sentinelle du dernier candidat mourait à chaque branche).

Le contrôle en feuille reste (« jamais un placement en violation ») ; il est désormais prouvablement inatteignable par `place` et épinglé par un test direct.
Les seuils comptent des crédits **du programme** : les cours préuniversitaires (0xxx) n'y entrent jamais — ni réussis (« scolarité préparatoire faite » ajoutait 20 crédits fantômes qui élargissaient les domaines et relançaient l'explosion), ni placés — comme le compteur de l'UI qui les garde hors du total du bac. La référence naïve du proptest applique la même règle.
`PROPOSE_MAX_NODES` passe de 200 k à 2 M : le plafond ne mord que sur les programmes durs (B-GMC broie ses agrégats ~3 s natif), où une proposition lente bat une grille vide.
Résultat mesuré : B-GMC-H27 passe de 0 placement en 20 M de nœuds à 38/43 cours placés en 140 ms au budget de l'UI.

## Alternatives rejetées

- **Monter le budget de nœuds** : 100× n'y suffisait pas — le rejet en feuille est exponentiel, pas une question de budget.
- **Plafond naïf `(s−1) × plafond`** : compte les étés fermés comme pleins et admet des sessions qu'aucune affectation n'atteint (é3 du B-GMC : son seul stage exige 42 cr).
