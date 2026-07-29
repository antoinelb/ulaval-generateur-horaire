# La semaine est un bitset à seaux de 5 minutes

Date : 2026-07-28

## Contexte

Le solveur A teste le chevauchement de plages des milliers de fois par recherche ; la conception (§4 de `solveur-conception.md`) retient un bitset fixe précalculé par option.
Restait à fixer la granularité et les cas limites de `slots_to_mask`.
La granularité a été mesurée sur **toutes** les données (2026-07-28) : 12 021 plages, 24 042 bornes, les 48 fichiers de `data/cours/` (2009→2026).

## Décision

`WeekMask([u64; 32])` : 7 jours × 288 seaux de 5 minutes = 2016 bits (256 octets) ; `overlaps` = ET mot à mot, `merge` = OU, `EMPTY`/`is_empty`.

- **Seaux de 5 minutes — nécessaires et suffisants.** 299 bornes réelles ne tombent pas sur une grille de 10 minutes (ENS-2003 10:45–12:15, CAT-1005 09:00–11:45, MDD-6016 11:45–13:15, 44 cours en tout) : des seaux de 10 minutes créeraient de faux conflits en dos-à-dos. Seules 3 bornes échappent à la grille de 5 minutes, toutes « 23:59 » (SIN-3150 h2022, plages « journée entière » 07:00–23:59) — couvertes par l'arrondi ci-dessous. La conception disait « 0 contre-exemple sur les 3975 plages d'a2026 » : vrai pour a2026 seul, l'historique complet en porte 3.
- **Indice de seau** = `jour × 288 + heure × 12 + minute / 5`, bits posés sur l'intervalle **demi-ouvert** `[début, fin)` : deux cours dos-à-dos partagent un instant, pas un seau.
- **Minutes hors grille arrondies vers l'extérieur** : plancher au début, plafond à la fin — du temps occupé n'est jamais déclaré libre (23:59 bloque jusqu'à 24:00, dernier bit 2015, aucun débordement possible).
- **Plage inversée (début ≥ fin) = aucun bit** : c'est l'intervalle demi-ouvert vide, pas un cas spécial ni une erreur — rien dans `core` n'interdit ce `Slot`, et `slots_to_mask` reste totale.
- **Section sans plage (à distance) = masque vide** : elle ne conflicte jamais.
- Construction seau par seau (`flat_map` + `fold`), pas d'arithmétique de masques par mot : une plage réelle fait ≤ ~36 seaux et les masques se construisent une fois par option — l'évidence prime sur l'astuce, et le chevauchement de mots u64 est correct par construction.

## Alternatives rejetées

- **Liste d'intervalles par option** : le test de chevauchement redevient O(n²) par paire et se paie au cœur de la recherche ; le bitset le rend constant et sans branche.
- **Résolution à la minute** (10 080 bits) : 5× la mémoire pour une précision que 24 039 bornes sur 24 042 n'utilisent pas, et les 3 restantes sont couvertes par l'arrondi.
- **Seaux de 10 minutes** : 299 bornes réelles hors grille — de faux conflits garantis sur 44 cours.
- **Erreur sur plage inversée** : la signature perdrait sa totalité (`-> Result`) pour un cas que les données scrapées ne produisent pas ; l'intervalle vide est la sémantique naturelle du demi-ouvert.
