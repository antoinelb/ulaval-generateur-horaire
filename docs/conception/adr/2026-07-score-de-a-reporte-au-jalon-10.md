# Le `Score` de A reporté au jalon 10 : `best_schedule -> Option<Schedule>`

Date : 2026-07-29

## Contexte

`docs/next_steps.md` et la conception §4 donnaient à A la signature `best_schedule(&[Vec<Opt>]) -> Option<(Schedule, Score)>`.
Or la sémantique de `Score` est celle des préférences (matins libres, pause dîner, journées compactes), explicitement reportée au jalon 10 et à calibrer contre des données réelles (ADR `2026-07-preferences-de-a-reportees-au-jalon-10`) — le même ADR établit qu'une signature sans sémantique implémentée et testée ne peut pas exister dans ce dépôt (`make test` exige 100 % de couverture, clippy `-D warnings` refuse le code mort).
Le contrat UI gelé n'a besoin de rien de plus que la règle déterministe « premier horaire faisable » (ADR `2026-07-contrat-horaire-hebdomadaire-vers-ui`).

## Décision

- `best_schedule(&[Vec<Opt>]) -> Option<Schedule>` : la première feuille de l'énumération — cours dans l'ordre d'entrée, options dans l'ordre du snapshot — qui **est** la règle gelée du contrat.
- `Score` naît au jalon 10, avec les opérations sur bits de préférence dont il est la somme ; la signature reprendra alors la paire prévue.
- `enumerate` collecte malgré tout **toutes** les feuilles valides dès maintenant : le classement du jalon 10 en a besoin, et l'espace est minuscule (a2026 : 1,21 option/cours en moyenne, n ≈ 5 cours/session).

## Alternatives rejetées

- **Un `Score` provisoire mais réel** (p. ex. nombre de seaux occupés) : une sémantique inventée aujourd'hui pré-empterait la calibration du jalon 10 et fuiterait dans B (le score entre dans son classement) avant d'avoir un sens.
- **`Option<(Schedule, ())>`** : la forme sans le fond — le tuple ne documente rien et impose un `.0` partout.
- **Reporter aussi `best_schedule`** : B (veto + score) et le harnais du jalon 2 ont besoin du point d'entrée dès maintenant ; seule la moitié « score » manque de sémantique.
