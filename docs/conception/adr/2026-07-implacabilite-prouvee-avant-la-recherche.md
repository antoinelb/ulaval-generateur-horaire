# Implaçabilité prouvée avant la recherche (champ `blocked`)

## Contexte

Sur le bac GEX réel, un cours au préalable insatisfiable (GEX-3333 exigeant GCI-1011, écarté faute d'offre avant `bc5f0fb`) rendait l'instance infaisable, mais la recherche brûlait les 10 M de nœuds du budget sans conclure : le cours fautif étant ~30ᵉ dans l'ordre d'entrée, le DFS redécouvrait la même impasse sous chaque arrangement des cours placés avant lui (mesuré : 10,6 s en release, verdict « budget épuisé — ensemble partiel »).
Un verdict `False` d'une feuille de préalable peut être permanent (code universitaire ni listé ni réussi, cours son propre préalable, seuil de crédits au-dessus de tout ce qui existe) : aucune affectation ne le répare.

## Décision

Avant `search()`, chaque candidat passe un filtre : domaine vide, ou arbre de préalables `False` sous l'évaluation optimiste (toute feuille satisfiable par au moins une affectation est tenue pour satisfaite).
Un candidat retenu par le filtre prouve l'infaisabilité de l'instance entière en O(cours × arbre) : `place` retourne `Completion::Complete`, zéro solution, et un champ additif `Placement.blocked` nommant chaque cours fautif avec sa raison (`EmptyDomain` ou `UnsatisfiablePrerequisites`).
Le harnais affiche « Implaçables (prouvé avant recherche) : … » ; l'UI pourra faire de même.
Le filtre est correct mais volontairement incomplet : les impossibilités croisées (précédences mutuellement exclusives sans concomitance, nœuds de capacité) restent l'affaire de la recherche.
La référence `tests/reference/solveur_b/` est alignée sur l'ADR `2026-07-presomption-limitee-au-preuniversitaire` (feuille inconnue : préuniversitaire satisfaite, universitaire violée) et la fixture `unsatisfiable-prerequisite-proves-infeasible` gèle le cas.
La borne de crédits restants dans `expand` (crédits à placer > capacité résiduelle totale) complète le dispositif pour l'infaisabilité par capacité : le cap 12 du bac GEX passe de 7,8 s inconcluants à 0,02 s prouvés.

## Alternatives rejetées

- Chaîner le filtre (un candidat bloqué rend `False` les feuilles qui le nomment) : inutile au verdict — un seul candidat bloqué prouve déjà l'infaisabilité — et ne servirait qu'au diagnostic ; à revisiter si l'UI en a besoin.
- Propagation de domaines complète (forward checking, cohérence d'arc) : coût d'implémentation disproportionné tant que le filtre plus la borne de capacité couvrent les cas réels observés.
