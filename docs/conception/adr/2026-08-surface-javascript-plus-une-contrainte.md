# La surface JavaScript n'est plus une contrainte

**Date :** 2026-08-29
**Statut :** accepté (décision Antoine).

## Contexte

Le module WASM a été conçu pour un consommateur JavaScript précis : `../grille-de-cheminement-interactive`, le front Elm-vanilla qu'Antoine fait évoluer en parallèle, nourri par les endpoints publiés sur Pages (`/pkg`, `/data`) et par le registre CORRECTIFS-AMONT.
Huit exports `#[wasm_bindgen]`, des déclarations TypeScript dérivées par `tsify`, un guide du consommateur JavaScript dans le livre, et surtout une clause tacite sur chaque décision de format : *est-ce que la grille JS le lit encore ?*
C'est cette clause qui a fait poser la feuille `{"concomitant": …}` à côté de la chaîne nue plutôt qu'à sa place (`core/src/course.rs`), et qui a fait remonter des items du registre amont jusque dans les commentaires du solveur.

Depuis, l'app Dioxus porte la totalité des fonctionnalités : le consommateur JS n'est plus la cible d'une livraison, et le maintenir en phase coûte une contrainte de conception à chaque changement.

## Décision

- **La compatibilité avec `grille-de-cheminement-interactive` cesse d'être une contrainte.**
  Aucune décision de format, d'API ou de données ne se justifie plus par « le lecteur JS ».
- **Rien n'est supprimé du code ni du déploiement** : les huit exports, `schedule.rs`, `questions.rs`, les dérives `tsify`, la cible `make wasm`, `/pkg` et `/data` restent tels quels — gelés, servis au mieux, sans promesse.
  Ce qui existe continue de compiler, d'être linté sur la cible wasm et couvert à 100 % ; ce qui casserait un consommateur JS n'est plus un empêchement.
- **Le « Guide du consommateur JavaScript » sort du livre** (cinq chapitres) : documenter une surface qu'on ne s'engage plus à tenir promet ce qu'on n'honore pas. `architecture/frontiere-wasm.md` la décrit encore, comme code publié et non comme contrat.
- **Les commentaires cessent d'invoquer le dépôt amont** comme justification. La raison d'être d'un comportement est ce qu'il fait pour l'app, pas qui d'autre le lit.
- **Les formats de données ne bougent pas.** La feuille `concomitant`, l'union `Rule`, les clés `valid` absentes-quand-vraies restent ce qu'elles sont : les changer ferait churner `data/cours.json` et les fixtures pour rien.

Les ADR `2026-08-module-wasm-quatre-fonctions-js`, `2026-08-surface-wasm-etendue-a-huit-fonctions`, `2026-08-snapshot-en-cache-dans-le-module-wasm` et `2026-08-types-typescript-tsify-declaratif` restent la description exacte du code publié ; elles perdent seulement leur force de contrainte sur ce qui vient.

## Alternatives rejetées

- **Tout supprimer maintenant** (les huit exports, `schedule.rs`, `questions.rs`, `tsify`, `make wasm`, `/pkg`) : ~700 lignes en moins, mais ça casse sur-le-champ ce qui importe déjà le module par URL, pour une économie nulle — ces modules sont écrits, testés et ne freinent personne. La suppression reste possible plus tard, quand plus rien ne pointera vers `/pkg`.
- **Garder le guide dans le livre** : un guide est une promesse de tenue. Le retirer dit la vérité — la surface existe, elle n'est plus suivie.
- **Réécrire les formats libérés de la contrainte** (fusionner la feuille `concomitant` dans la chaîne, par exemple) : le gain est cosmétique, le coût est une re-dérivation de tout `data/cours.json` et de chaque fixture.
