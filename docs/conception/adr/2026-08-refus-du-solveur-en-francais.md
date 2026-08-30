# Le refus du solveur se dit en français, l'anglais reste derrière le repli

Date : 2026-08-30

## Contexte

Bernard, directeur de programme, ajoute sur la concentration « Eau et environnement » un cours (FOR-2020) qui satisfait à la fois la Règle 1 et la Règle 2, sans lui assigner de règle (constat du 2026-08-29).
L'écran affiche :

> ⚠ Le solveur n'a pas pu répondre — détail technique : Règle 1 (concentration scope) : the selection sums 15 credits, above the max 12 — semantics await the director's ruling

Son jugement : le problème lui-même est légitime et bien détecté, mais la présentation trahit le code sous-jacent — « scope », « semantics », « the director's ruling » sont du jargon de développeur, et rien ne dit quoi faire.

La cause est mécanique.
`crates/wasm/src/protocol.rs` renvoie `Response::Error { id, message }`, où `message` est un `to_string()` de `CoverageError`, `PlacementError` ou `IntakeError` — anglais, comme tout le code (`CLAUDE.md`).
`components/mod.rs` l'enrobait d'une phrase française et laissait le texte anglais **dans le message principal** :

```rust
AlertBody::Note(format!("Le solveur n'a pas pu répondre — détail technique : {message}"))
```

Ce qui viole ERR-1 (cinq parties, dont un « quoi faire maintenant ») et ERR-3 (le détail technique est toujours à un clic, jamais le message principal).
`UiError` portait déjà les cinq champs : le défaut était de ne pas s'en servir sur ce chemin-là.

Le même défaut existait à un deuxième endroit : la branche `other` de `panel::coverage_error_message` composait « … Détail : {other}. » avec l'anglais de `core` en clair dans le texte du panneau.

## Décision

**Aucun texte anglais dans un message principal.** Trois changements :

1. `present::present_solver_error(&str) -> UiError` traduit la chaîne du worker.
   Il reconnaît les deux familles atteignables au clic — `CreditsOverMax` et `CountOverMax` — en relisant la formulation exacte de `CoverageError`, et la vérification incomplète (`verification needs a session for every course left to place`).
   Tout le reste reçoit un enrobage français complet en cinq parties. Dans tous les cas, `detail` porte la chaîne brute, intacte.
2. Une seule formulation pour « cette règle dépasse son maximum », `present::present_over_max`, partagée par les deux portes : le `CoverageError` typé que compte le panneau, et la chaîne que renvoie le worker. `scope_origin` déménage de `panel.rs` vers `present.rs` avec elle.
   Le « quoi faire » nomme les deux issues réelles : retirer un cours de la règle, ou le rattacher à une autre avec le menu « entente avec la direction… » de sa ligne — c'est exactement l'arbitrage que `core` refuse de faire à la place de l'étudiant (ADR `2026-07-somme-au-dessus-du-max-en-erreur-typee`).
3. `PanelModel.coverage_error` passe de `Option<String>` à `Option<UiError>`, et les deux vues qui rendent une `UiError` (le toast, le panneau) exposent le texte technique dans un `<details>` « Détail technique » avec l'identifiant, jamais en tête.

La relecture de la chaîne est un couplage assumé, et **vérifié** : le test `an_over_max_from_the_worker_is_read_back_into_french` alimente le parseur avec le `Display` réel de `CoverageError`. Une reformulation de `core` casse la CI au lieu de dégrader silencieusement chaque dépassement en message générique.
Une forme que le parseur ne reconnaît pas — un `scope` inconnu, un nombre illisible, un suffixe différent — retombe sur l'enrobage générique plutôt que d'inventer une règle ou un chiffre; la chaîne brute reste dans `detail`, jamais perdue.

## Alternatives rejetées

- **Traduire chaque variante de `PlacementError`, `IntakeError` et `CoverageError` en français dans `core`** : les messages d'erreur du code sont en anglais (`CLAUDE.md`), et `core` sert aussi la surface JavaScript et le scraper — la langue de l'interface n'a pas à y remonter.
- **Faire porter à `Response::Error` une erreur structurée (code + champs) plutôt qu'une chaîne** : c'est la solution propre, mais elle demande de rendre `Serialize` trois énumérations d'erreur de `core` et de faire évoluer le protocole du worker pour un seul cas de présentation. Le couplage par chaîne, testé contre le `Display` réel, coûte un test et attrape la même régression.
- **Garder l'anglais en tête et ajouter seulement un « quoi faire »** : ERR-3 est explicite — le détail technique n'est jamais le message principal.
- **Ne corriger que le toast** : la branche `other` du panneau composait le même défaut; deux endroits qui affichent la même erreur ne peuvent pas avoir deux politiques.
