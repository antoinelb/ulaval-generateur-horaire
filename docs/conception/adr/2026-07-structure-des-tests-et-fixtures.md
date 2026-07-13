# Structure des tests : unitaires, intégration et fixtures

Date : 2026-07-10

> **Partiellement remplacé** (2026-07-13) : la cible externe `unit` est abandonnée — les tests unitaires vivent en ligne (`#[cfg(test)]` dans `src/`, exclus de la couverture), pour l'accès aux items privés et des chiffres de couverture honnêtes.
> Voir `2026-07-tests-unitaires-en-ligne-et-couverture.md`. La cible `integration` et l'organisation des fixtures restent en vigueur.

## Contexte

La crate `core` a maintenant une surface de types conséquente (`Course` et ses types imbriqués, énumérations, conversions `serde`).
Tous ses tests vivaient dans un seul fichier mêlant deux préoccupations : des tests de comportement sur données en ligne et un round-trip adossé aux vrais cas de test JSON.
Ces cas de test résidaient à la racine `tests/test_cases/`, consommés seulement par `core` via un chemin relatif, sans distinction entre fixtures partagées et fixtures propres à une crate.
Il faut une structure qui monte en charge quand `scraper` et `ui` auront leurs propres tests.

## Décision

**Deux cibles `[[test]]` dans `core`**, séparant les tests par nature d'entrée :

- `unit` (`tests/unit/`) — un type à la fois, données en **littéraux JSON en ligne** ; épingle le comportement (`serde` `rename`/`default`/`untagged`, validation aux frontières, rejets et cas limites que les fixtures valides ne couvrent jamais).
- `integration` (`tests/integration/`) — round-trip adossé aux **fixtures réelles** ; épingle la conformité des types aux données que le scraper produira.

Chaque dossier a un `main.rs` qui déclare ses sous-modules (`mod course;`), car Cargo n'auto-découvre que les `.rs` à la racine de `tests/`, pas dans les sous-dossiers ; d'où une entrée `[[test]]` explicite par cible.

**Réorganisation des fixtures sous un parapluie `tests/fixtures/`** :

- commun, utile à toutes les crates : `tests/fixtures/test_cases/{classes,programs,listing}/` (déplacé depuis `tests/test_cases/` par `git mv`, historique préservé) ;
- propre à une crate : `crates/<crate>/tests/fixtures/` (pour `core`, un `.gitkeep` en attendant des fixtures spécifiques, p. ex. un cours mal formé pour tester la gestion d'erreur).

## Alternatives rejetées

- **Renommage à plat `tests/test_cases/` → `tests/fixtures/`** : perd le regroupement `test_cases` et entre en collision avec les futures fixtures HTML gelées, déjà nommées « fixtures » dans les docs ; le parapluie laisse la place à un futur `tests/fixtures/html/`.
- **Garder `tests/test_cases/` inchangé** : pas de dossier commun `tests/fixtures/` littéral tel que demandé, et pas d'emplacement symétrique clair pour le spécifique par crate.
- **Tests unitaires dans `src/lib.rs` via `#[cfg(test)]`** : tous les types sont `pub`, donc un test externe vérifie aussi que l'API publique est utilisable, et garde `lib.rs` concentré sur le domaine.
- **Fixtures par fichier pour l'unitaire** : indirection inutile pour des entrées minuscules ; les littéraux en ligne se lisent sur place, à côté de l'assertion.
