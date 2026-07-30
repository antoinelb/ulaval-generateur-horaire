# Schéma des fixtures de placement (`organigrammes/`)

Date : 2026-07-29

## Contexte

Le format JSON de l'organigramme était une question ouverte du plan (« provisoire »).
Écrire les fixtures avant le solveur force une forme concrète : elle devient le contrat provisoire que `organigramme.rs` consommera.
B énumère toutes les solutions (`2026-07-b-enumere-toutes-les-solutions`), donc l'attendu peut être un ensemble comparé par égalité exacte — l'ordre de parcours (`cheminement_type`) n'y change rien et n'apparaît pas dans les fixtures.

## Décision

```json
{
  "sessions": ["fall", "winter"],
  "credit_cap": 17,
  "concomitant": false,
  "passed": ["MAT-1900"],
  "pinned": { "GCI-1007": 2 },
  "courses": [ … ],
  "expected": { "complete": true, "solutions": [ { "GCI-1007": 2 } ] }
}
```

- `sessions` : saisons ordonnées des sessions 1..n ; tout numéro de session est un indice 1-based dans ce tableau.
- `credit_cap` : **entrée de fixture, jamais une constante** — la valeur réelle (17 ?) est une question ouverte à trancher avec le directeur ; seule la mécanique (rejet au dépassement) est figée ici.
- `concomitant` (défaut `false`) : l'option globale documentée qui relâche « strictement avant » en « avant ou identique ».
- `passed` : cours réussis — retirés du placement, crédits précomptés vers les seuils `program_credits`, satisfont les feuilles de préalables ; leur `Course` complet figure dans `courses`.
- `pinned` : code → numéro de session, réduction du domaine à un singleton.
- `courses` : les `Course` complets de tous les cours, à placer comme réussis.
- `expected.complete` : `true` = recherche épuisée, l'ensemble est complet — avec `solutions: []`, l'infaisabilité est **prouvée** ; `false` est réservé au cas « budget de nœuds atteint », jamais fixturé tant que le comptage de nœuds n'est pas décidé, pour que les deux issues ne soient jamais confondues.
- `expected.solutions` : chaque solution = objet code → numéro de session (les réussis n'y figurent pas) ; ordre canonique — clés triées, tableau trié lexicographiquement par la séquence (code, session) — que le harnais Rust appliquera à la sortie en ordre de recherche avant comparaison.

Phénomènes volontairement non fixturés, chacun figeant une décision non prise :

- budget de nœuds → ensemble partiel (comptage non défini, égalité exacte impossible sur un ensemble partiel) ;
- concomitance par arête (jetons ombrés des organigrammes) : `PrereqTree` ne la porte pas — l'astérisque ne survit que dans `prerequisites.raw` ; l'encoder est une décision scraper/modèle de données ;
- préalable vers un cours hors liste et non réussi (p. ex. collégial) : bloquer, remonter ou présumer satisfait à l'admission n'est pas documenté — les fixtures évitent le cas et la référence erre dès que le verdict en dépend (une branche `any` déjà satisfaite tolère un code inconnu : ECN-2901 au côté d'ECN-4901 réussi) ;
- `Credits::Range` en placement : la pondération choisie est une question ouverte (`2026-07-resolution-des-credits-choisis`).

## Conséquences

Treize fixtures gelées le 2026-07-29, un phénomène chacune, comptes de solutions mesurés par la référence : chaîne de préalables (1), saison restrictive (1), plafond scindant (2), seuil `program_credits` (5), réussi-satisfait-préalable (1), crédits réussis précomptés (1), épinglage respecté (1), épinglage contre l'offre (0, prouvé), sur-contrainte (0, prouvé), ensemble cartésien (4), concomitance globale (1 — le mode strict donne 0), veto hebdomadaire scindant (2), inversion des projets intégrateurs en admission hiver (1 : `{"GEX-3333": 2, "GEX-3335": 1}`, 30 cours réussis réels, 87 crédits).
La fixture contingente du bac complet en admission automne n'est **pas livrée** : l'énumération brute de la référence, sans élagage de précédence, est intraitable sur 34 cours × 8 sessions — la mesure du nombre de solutions du bac complet reste au Verify de la Phase 3 (`docs/next_steps.md`), exécutée par le solveur lui-même.

## Alternatives rejetées

- **Un plafond de crédits codé en dur dans les fixtures et le solveur** : fige un chiffre sans source ; en entrée de fixture, la mécanique se teste avec des petits plafonds synthétiques sans préjuger de la vraie valeur.
- **Attendre seulement la première solution ou un compte** : n'épingle pas la complétude — le contrat de B est l'ensemble entier, prouvé complet quand la recherche s'épuise.
- **Sessions numérotées avec saison implicite (parité)** : casse dès qu'une session d'été ou un puits « étranger » s'insère ; la liste explicite de saisons porte tous les cas.
- **Inclure le `cheminement_type` comme seed** : il n'influence que l'ordre de la recherche, pas l'ensemble ; l'inclure suggérerait le contraire.
