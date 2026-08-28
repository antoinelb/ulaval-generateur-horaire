# Les électifs choisis sous un bloc partent avec lui

Date : 2026-08-27

Amende `2026-08-electifs-orphelins-purges-au-changement-de-bloc`, et **renverse** son alternative rejetée « suivre qui a ajouté l'électif » (décision d'Antoine, 2026-08-27).

## Contexte

Rapport étudiante 2026-08-27 (F4) : Élodie choisit FOR-2020 sous « Génie du bâtiment », puis passe au bloc neutre.
La Règle 1 du bloc neutre liste FOR-2020 : le cours n'est donc **pas** orphelin, `scope_orphans` ne le voit pas, et il reste compté sous un bloc qu'elle n'a jamais parcouru.
Le total ne ment pas au sens strict — le cours est bien couvert — mais il répond à une question qu'elle n'a pas posée.

La couverture seule ne peut pas trancher : elle ignore *sous quel bloc* l'étudiante a fait son choix.

## Décision

Au changement de concentration ou de profil, **tout** électif choisi sous le bloc quittant part avec lui, même s'il reste admissible.

- `Plan.elective_origins : BTreeMap<String, String>` — code → « c » (concentration) ou « f » (profil). Le champ profite du `#[serde(default)]` du struct : une vieille sauvegarde `gh.v1.plan` restaure sans étiquettes.
- `state::tag_elective_origin(plan, code, origin)` : `None` est un non-acte — un glissé au ruban ou un déplacement dans la grille n'est pas un choix et n'écrase aucune étiquette. `place_course` restaure l'étiquette que sa propre purge vient d'effacer (`state::elective_origin`).
- `panel::section_origin(key)` lit le préfixe de la section (« c/… », « f/… », « p/… »), les mêmes que `rule_grants` ; il se propage `SectionView` → `RowView` → `CourseChoice`, et le formulaire manuel le tire de la règle qu'il attache.
- `panel::block_departures(...)` = `scope_orphans(...)` ∪ `scoped_electives(plan, prefix)`, dédupliqué et trié. `scope_orphans` **reste** : c'est le filet des plans sans étiquettes.
- Le retrait se fait dans le même `edit_plan` que le changement de bloc — un seul « Annuler » — et est annoncé : « Cours retirés avec l'ancien bloc : … ».

## Alternatives rejetées

- Garder la couverture seule (l'ADR amendé) : c'est exactement le cas FOR-2020, recompté en silence.
- Demander à l'étudiante quoi garder : un dialogue de confirmation, interdit par ACT-2 ; l'acte est déjà réversible d'un clic.
- Étiqueter aussi les électifs que le solveur injecte : ils ne sont le choix de personne, et les faire partir avec un bloc les ferait réinjecter aussitôt.

## Conséquences

- Un plan importé par lien de partage n'a **pas** d'étiquettes : la trame `ShareV1` est gelée avant déploiement (`2026-08-trame-de-partage-unique-avant-deploiement`) et le champ n'y entre pas. Ces électifs sont jugés par la couverture seule, comme avant.
- Un cours retiré perd son étiquette avec le reste (`purge_codes`) : le reprendre plus tard est un choix neuf, sous le bloc regardé à ce moment-là.
