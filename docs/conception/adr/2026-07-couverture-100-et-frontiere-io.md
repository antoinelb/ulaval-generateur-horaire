# Couverture à 100 % : frontière IO exclue, logique pure mesurée

Date : 2026-07-17

## Contexte

`make test` visait 100 % de couverture, mais trois catégories de lignes restaient inatteignables par des tests honnêtes :

- les `main.rs` des binaires (`scraper`, `ui`) — points d'entrée qui lancent l'application ;
- les branches d'échec des assertions dans les modules `#[cfg(test)]` (le bras `false` d'un `matches!` ne s'exécute que si le test échoue) ;
- les instanciations mortes de llvm-cov : les closures d'erreur du parseur, jamais exécutées dans le binaire d'intégration (fixtures toutes valides), comptaient comme régions manquées fantômes malgré une couverture source complète.

Les branches tty de `print::write` semblaient aussi intestables, mais ne l'étaient pas : elles branchent sur `state.is_tty`, un simple booléen.

## Décision

- **`main.rs` exclus de la mesure** : le regex du makefile devient `(lib\.rs|/mod\.rs|/main\.rs)$`. Les binaires restent minces (frontière `anyhow`), leur logique vit dans les modules mesurés.
- **Modules de test exclus d'eux-mêmes** : `#[cfg_attr(coverage_nightly, coverage(off))]` sur chaque `mod tests` (patron déjà présent dans `print.rs`) ; la `feature(coverage_attribute)` est déclarée `all(coverage_nightly, test)` pour ne pas déclencher `unused_features` en build normal.
- **`print::write` scindé** plutôt qu'exclu : la construction de la sortie devient la fonction pure `render_output(state, permanent) -> String`, testée avec `is_tty: true/false` (codes ANSI affirmés exactement) ; `write` ne garde que `print!` + `flush`.
- **Chemins d'erreur exercés aussi en intégration** : un test `error_paths_stay_errors_through_the_public_api` fait tourner les closures d'erreur dans le binaire d'intégration, éliminant les instanciations fantômes.

## Alternatives rejetées

- **`coverage(off)` sur `write` entier** (première tentative) : masquait des branches en réalité testables — un 100 % par non-mesure ; retiré au profit de la scission pure/IO.
- **Tests appelant `write` avec un état tty sans rien affirmer** : du théâtre de couverture — exécute les lignes, ne vérifie rien.
- **Accepter 99,78 %** : défendable (l'artefact d'instanciation ne cache aucune ligne non testée), mais le test d'intégration des chemins d'erreur a une valeur propre : il vérifie le contrat d'erreur à travers l'API publique.
