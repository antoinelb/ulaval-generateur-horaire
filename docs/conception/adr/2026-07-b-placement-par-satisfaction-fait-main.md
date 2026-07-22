# B est un solveur de placement par satisfaction, fait main — il ne choisit jamais de cours

**Date :** 2026-07-21
**Statut :** accepté ; ferme l'embranchement ouvert le même jour par `2026-07-moteur-de-b-embranchement-pumpkin`.
Sortie révisée le jour même par `2026-07-b-enumere-toutes-les-solutions` (toutes les solutions retournées, pas seulement la première) ; le reste tient.

## Contexte

L'embranchement du moteur de B dépendait de deux questions : « B est-il de la satisfaction ou de l'optimisation ? » et « l'apprentissage de Rust s'applique-t-il à B ? ».
Décision produit (Antoine) : **satisfaction** — et, plus structurant encore, B ne doit pas choisir de cours.
Recommander des cours à option n'a pas de sens en pratique (la règle 5 GEX, wildcard `Keyword::Any` sur tout le catalogue, en était déjà la preuve par les données) ; l'étudiant — ou le directeur qui bâtit une base générale — fournit la liste, possiblement partielle, et l'UI montre les règles non couvertes et les cours candidats qui rentrent dans l'horaire ouvert (là, `weekly::is_feasible` suffit : aucun cheminement à vérifier).
Une fois les électifs choisis, la sélection doit être validée contre le programme — mais ce n'est pas le travail du solveur de placement.

## Décision

- **B prend une liste de cours et la place** sur les sessions selon préalables (ET/OU), offre par saison, plafond de crédits, contraintes utilisateur (singletons) et le veto horaire de A — sans jamais choisir de cours.
  La couche « sélection » disparaît de B (plus de `selected[c]`, plus d'énumération d'ensembles satisfaisant les règles).
- **La couverture et la validation des règles sont une fonction pure séparée de `core`** (invariant « aucune logique métier dans la vue »), consommée par l'UI : rapport satisfait / à combler / candidats, candidats filtrés par `weekly::is_feasible` contre l'horaire ouvert de la session visée.
- **Satisfaction par recherche complète** : affectation systématique cours par cours, ordre de valeurs = session du `cheminement_type` de référence d'abord (sans seed : plus tôt offerte), budget de nœuds explicite.
  « Aucun cheminement faisable » n'est affirmé que sur recherche épuisée ; un budget atteint rapporte « budget épuisé » — jamais confondus.
  Rejet, jamais réparation.
- **Moteur fait main en `core` pur ; pas de spike, aucune dépendance solveur.**
  Pumpkin (`pumpkin-core`) reste le repli documenté (conception §5.2–§6, faits WASM vérifiés le 2026-07-21 toujours valables) si la recherche main venait à thrasher.

## Alternatives rejetées

- **Optimisation (objectif pondéré)** : fausse précision contre des poids estimés ; l'utilisateur édite le résultat de toute façon ; un ordre de recherche sensé (seed d'abord) donne un premier placement naturel sans objectif.
- **B choisit les électifs** : sans sens produit, non énumérable pour les règles `Keyword` (`Any`, `Negotiated`), et contraire à « remonté, jamais inventé ».
- **Génération par perturbations du seed seulement** : incomplète — un « infaisable » non prouvé serait une perte silencieuse déguisée en réponse.
- **Garder le spike Pumpkin** : n'apporte plus rien une fois la sélection retirée et la satisfaction décidée — l'instance restante est du placement pur, petit ; un jour de spike n'achèterait aucune information qui change la décision.
- **Validation des règles côté UI** : brise l'invariant « toute la logique métier en `core` pur ».
