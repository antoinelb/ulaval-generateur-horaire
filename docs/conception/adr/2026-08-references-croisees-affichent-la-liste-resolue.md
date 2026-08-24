# Les références croisées affichent la liste résolue par le cœur

Date : 2026-08-23

## Contexte

`coverage_report` résolvait déjà une `RuleCourses::Reference`, mais le panneau n'affichait des rangées que pour une `RuleCourses::List`.
Les Règles 2 des concentrations spécialisées du B-GCI apparaissaient donc vides malgré un comptage valide.

## Décision

Le cœur expose `resolved_rule_courses(program, rule)` et le panneau l'utilise comme `coverage_report`.
La liste cible conserve son ordre, les rangées sont dédupliquées et le texte `raw` de la référence reste visible.
Une cible absente, non-liste ou elle-même référencée demeure une erreur typée.

## Alternatives rejetées

- **Aplatir les données scrapées** — la provenance de la référence serait perdue et deux consommateurs pourraient encore diverger.
