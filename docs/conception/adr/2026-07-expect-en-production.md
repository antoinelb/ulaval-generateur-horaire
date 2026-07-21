# `expect` en production : `?` quand un canal d'erreur existe, `LazyLock` sinon

Date : 2026-07-19

## Contexte

Un audit des `expect`/`unwrap`/`panic!` du dépôt en dénombre environ 160, dont 9 seulement en code de production : le reste vit sous `#[cfg(test)]` ou dans `tests/`, où l'abandon immédiat est le comportement voulu (un test incapable de monter son fixture doit échouer bruyamment), et les `.expect(1)` de `wiremock` sont des assertions de mock, pas des panics.

Les 9 restants ne relèvent pas du même problème.
Certains paient une panic là où un `?` gratuit existe ; d'autres affirment une invariance que le système de types ne peut pas exprimer, faute d'API `const` en amont ; un seul repose sur une invariance réellement fragile.

## Décision

Trois règles, appliquées dans cet ordre.

**1. Si la fonction englobante renvoie déjà `Result`, utiliser `?`.**
Concerne `cli.rs:74` (`Fetcher::new`) et `cli.rs:96` (`serde_json::to_string_pretty`), tous deux dans des fonctions `anyhow::Result` dont les types d'erreur se convertissent seuls.
L'affirmation portée par l'`expect` est vraie, mais ce n'est pas l'argument : le canal d'erreur ne coûte rien, donc payer une panic pour l'éviter est strictement moins bon.
C'est en plus une suppression nette, l'`expect` et le commentaire qui le justifie disparaissant ensemble.

**2. Si la panic est inévitable, la sortir du chemin chaud avec `std::sync::LazyLock`.**
Concerne les 6 `Selector::parse(...).expect("Static selector is valid")` de `parser/catalogue.rs`.
`scraper` n'expose aucun parseur de sélecteur `const` : `?` ne ferait que propager une erreur impossible, et la panic reste la formulation honnête de « ce littéral est du CSS valide ».
Le vrai défaut est ailleurs — `parse_catalogue` reparse deux sélecteurs à chaque entrée, soit ~10 000 fois par scrape.
`LazyLock` (stable depuis Rust 1.80, aucune dépendance) traite les deux : une seule initialisation, et les constantes `_str` restent disponibles pour les messages `ParseError` qui les utilisent déjà.

**3. Si l'invariance est porteuse et que la retirer coûte une restructuration, la documenter et la garder.**
Concerne `fetch.rs:144` :

```rust
Err(last_error
    .expect("`max_attempts` >= 1 so the loop body should have set it"))
```

C'est le seul cas où l'affirmation est réellement fragile : elle ne tient que parce que *chaque* branche `continue` d'un corps de boucle d'une soixantaine de lignes assigne `last_error` au préalable.
Une septième branche qui l'oublie fait paniquer le scraper en production sur un simple incident réseau.
L'éliminer suppose de faire de la dernière tentative l'expression finale de la fonction plutôt qu'un `Option` accumulé, c'est-à-dire d'extraire la décision de réessai dans un `enum Outcome { Done | RetryAfter | Fatal }` et de sortir la dernière tentative de la boucle : environ 30 lignes de remaniement pour supprimer un `expect`.
Reporté tant que `fetch.rs` n'est pas retouché pour une autre raison ; à faire à ce moment-là.

## Application

Règle 2 appliquée le 2026-07-19 aux deux parseurs, `parser/catalogue.rs` et `parser/course.rs` — ce dernier n'était pas nommé ci-dessus mais portait 24 sites d'appel pour 22 sélecteurs distincts, dont ceux de `plage_field`, reparsés deux fois par appel et trois fois par plage horaire.
Chaque sélecteur devient un `static LazyLock<Selector>` déclaré au-dessus de sa fonction, et un unique `sel()` privé par fichier porte l'`expect`.
`course.rs` concentrait déjà le sien dans un tel `sel()` : le gain y est le nombre d'exécutions, 24 analyses par page devenant 22 par processus.
`catalogue.rs` passe lui de 7 sites d'`expect` à 1.

L'hypothèse que `LazyLock<Selector>` compile a été vérifiée avant d'être retenue : `static X: LazyLock<Selector>` exige `Selector: Send + Sync`, ce que `scraper` 0.27 satisfait.

Effet de bord à connaître pour la couverture : chaque fermeture `LazyLock::new(|| …)` est un enregistrement de fonction distinct pour llvm-cov.
Un sélecteur qu'aucun test ne déréférence devient donc une région non couverte — ce qui est la bonne alerte, puisqu'il s'agit alors d'un sélecteur mort à supprimer.

Règle 3 (`fetch.rs:144`) reste reportée : ce fichier n'a pas été retouché.

## Alternatives rejetées

- **Bannir `expect` en production sans exception** : forcerait `parse_matieres` et `parse_catalogue` à renvoyer un `ParseError` que rien ne peut produire, contaminant leurs appelants d'un cas mort — la panic dit la vérité, l'erreur ment.
- **Étendre la règle aux tests** : `unwrap_or_else(|e| panic!(...))` y est déjà le style maison (ADR `2026-07-tests-unitaires-en-ligne-et-couverture`) et porte un message de diagnostic ; un `Result` de test n'apporterait qu'un `?` silencieux.
- **Restructurer `fetch.rs` tout de suite** : le ratio remaniement/bénéfice est mauvais tant qu'aucune septième branche n'est en vue, et la boucle est couverte à 100 %.
