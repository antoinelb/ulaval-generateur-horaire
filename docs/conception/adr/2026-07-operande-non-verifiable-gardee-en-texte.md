# Une opérande que le planificateur ne peut pas vérifier est gardée en texte

Date : 2026-07-20

## Contexte

Quatre pages ont produit la même anomalie sous quatre habits :

```
ESP-1000 : ESG-2020 à 3799 OU Examen Classement en espagnol avec résultat de 5 à 8 OU Examen Test espagnol avec résultat de 5 à 5
FRN-1904 : Examen Test français Laval-Montréal avec résultat de 060.0 à 100.0
GCI-2510 : Examen Formation obligatoire stage avec résultat de P
FRN-1112 : FRN-1910 OU FRN 19543
```

- un **examen** — un résultat à obtenir à un test de classement ou à une formation obligatoire ;
- une **plage de cours** — `ESG-2020 à 3799` sans « Crédits exigés » derrière ne borne pas un décompte, elle nomme les cours eux-mêmes, dont un seul satisfait le préalable ;
- un **sigle fautif** — `FRN 19543` est du texte nu là où la page lie tout sigle existant ; ni `FRN-1954` ni `FRN-1543` n'existent au catalogue, la coquille est irréparable ;
- et, de la même famille, la **prose** (« Autorisation de la direction »).

Chaque fois, `parse_prereq_tree` renvoyait `Err` et **toute** l'expression basculait en `Prerequisites::Raw` : ESP-1000 perdait ses trois opérandes, FRN-1112 perdait `FRN-1910`.

Une première version distinguait les formes *reconnues* (examen, plage), admises dans l'arbre sans anomalie, des formes *inconnues* (sigle fautif, prose), qui restaient des erreurs.
La distinction ne tient pas : elle revient à demander « avons-nous écrit une règle pour cette chaîne ? », pas « savons-nous la vérifier ? ».
Elle était même perverse — `GEX, Crédits exigés : soixante` échouait parce qu'elle ressemblait *presque* à une forme connue, si bien qu'un texte moins reconnaissable s'en tirait mieux.

## Décision

`PrereqTree` gagne une variante `Raw { raw: String }`, sérialisée `{"raw": "…"}`.

Le tokenizer ne reconnaît que les formes que le planificateur sait **vérifier** — trois écritures de l'exigence de crédits, et le sigle de cours.
Tout le reste est gardé en texte, par défaut et non par énumération : `classify_operand` est un `unwrap_or_else` sur `checkable_operand`.
Il n'y a donc aucune règle « examen », aucune règle « plage » : ces formes ne correspondent à aucune forme vérifiable, ce qui suffit.

**Ce n'est pas une anomalie.**
Rien n'a échoué : la source demande quelque chose qu'aucun catalogue ne peut vérifier, et le snapshot le dit fidèlement.
L'invariant du plan est respecté par le nœud lui-même — « préalable hors grammaire → `{"raw": "…"}` ; affichés et comptés, jamais ignorés » (`docs/project_plan.md`) : la feuille est dans les données, l'UI l'affiche, l'étudiant tranche.
Un solveur ne peut ni la satisfaire ni l'ignorer ; il la traite comme « à confirmer par l'étudiant ».

ESP-1000 donne `{"any": [{"raw": "ESG-2020 à 3799"}, {"raw": "Examen Classement…"}, {"raw": "Examen Test…"}]}` et FRN-1112 `{"any": ["FRN-1910", {"raw": "FRN 19543"}]}` — le sigle valide à côté de la coquille.

Seule la **structure** de l'expression échoue encore en bloc — parenthèse non fermée, opérateur sans opérande, texte vide — parce qu'elle n'a aucune réparation locale : rien ne dit quelles opérandes un groupe ouvert devait contenir.
`Prerequisites::Raw` ne sert plus qu'à cela.

Effets de bord assumés :

- GCI-2510 n'est plus « la seule page gelée qui parse avec une anomalie » ; son préalable n'était pas de la prose, c'était un examen. Toutes les fixtures rejoignent la table commune.
- Le journal d'anomalies redevient « quelque chose a mal tourné » et disparaît quand la course est propre — le signal que l'ADR `2026-07-echec-de-page-cours-non-bloquant` visait.
- Le cache accepte ces pages, qui n'étaient plus jamais mises en cache (`crates/scraper/src/course.rs`, « only a clean parse is cached »).
- **Contrepartie** : une forme de préalable inédite apparaissant dans trois cents cours ne s'annoncera plus. Elle sera comptable dans les données — `grep -c '"raw"' data/cours/*.json` — mais rien ne la signalera au moment de la course. Si cela devient un problème, une ligne de résumé en fin de course suffira ; rien ne le justifie aujourd'hui.

## Alternatives rejetées

- **Une variante typée par examen** (`Exam { test, minimum }`) : il faudrait lire « avec résultat de 060.0 à 100.0 », « de 5 à 8 », « de P » — trois échelles sur trois exemples, pour un champ qu'aucun consommateur ne sait vérifier.
- **Développer une plage en `any` des cours qu'elle contient** : le parseur ne voit qu'une page à la fois et ignore quels sigles de la plage existent.
- **Garder l'opérande inconnue fatale à l'expression** : c'est perdre `FRN-1910` pour la coquille d'un autre sigle, et recommencer à chaque forme inédite du catalogue.
- **Garder l'opérande inconnue dans l'arbre mais la signaler quand même** : c'est la version intermédiaire écrite puis retirée le jour même — elle réintroduisait par la bande la distinction « avons-nous codé cette chaîne ? », et laissait le journal se remplir d'une coquille que personne ne peut corriger.
- **Distinguer dans l'arbre les deux sortes de `Raw`** : aucun consommateur n'en ferait quoi que ce soit, l'UI les affichant pareillement.
