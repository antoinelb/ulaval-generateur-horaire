# Une règle « Hors programme » accueille les cours pris ailleurs

Date : 2026-08-26

## Contexte

Un cours pris en dehors du programme n'avait aucun domicile.
`Plan.rule_grants` — l'entente avec la direction (ADR `2026-08-entente-cours-regle-et-scolarite-preparatoire`) — ne sait rattacher un cours qu'à une règle *existante*, et `panel::grantable_rules` n'offre que ce que le répertoire a écrit.
Un cours hors programme n'apparaissait donc dans aucune section du panneau ; crédité, il ne récoltait qu'un avertissement (`panel::unlisted_credited` : « est crédité mais n'apparaît dans aucune règle de ce programme »), sans aucun geste pour le résoudre.

## Décisions

- **Une règle « Hors programme », vide, sur tout programme.** `RuleCourses::List` sans aucun sigle : elle ne se remplit que par entente.
  Une liste vide donne gratuitement le comportement voulu — `browses_catalogue` ne rend `free` que pour `Keyword::Any`, donc la section n'offre ni rangée ni recherche par défaut.
- **Sans contrainte.** Il n'y a rien à vérifier : combien de cours un étudiant prend hors de son programme n'est pas une exigence. `core::rules` la range donc en `reported` avec son découpage (ADR `2026-08-regle-sans-contrainte-comptee-mais-reportee`), badge « — » (depuis remplacé par le nombre de cours qu'elle abrite — ADR `2026-08-le-panneau-ne-repete-pas-ce-qu-il-sait-deja`), et — n'étant pas contrainte — elle ne réclame aucun code dans `scope_reports` : insérée n'importe où, elle ne déplace pas un verdict.
- **`credits_in_addition: true`.** Un cours hors du programme ne paie pas le diplôme : `wasm::credits::en_sus_codes` le range dans `in_addition`, jamais dans `counted`. Le badge ne comptant que les cours, c'est la **note** de la règle qui porte ce fait à l'écran (TRU-1).
- **Injectée au chargement de l'UI**, `data::with_out_of_program`, jamais écrite dans `data/programmes/*.json` : la règle énonce un fait de l'étudiant, pas du répertoire.
  Deux points d'appel, parce que `Snapshot.programs` a deux écrivains : `parse_data` (livrés + importés, URL comme fichier) et `add_local_program` (import à chaud). La copie poussée gagne la règle, `LocalProgram.program` reste tel qu'importé — c'est lui que `persist::encode_local_programs` réécrit, et une copie persistée porteuse de la règle en gagnerait une seconde au chargement suivant.
- **Placée juste avant « Scolarité préparatoire »**, donc après « Stages ». Le panneau extrait la préparatoire et la rend seule tout en bas (`grouped_sections`) : la règle est ainsi la dernière du groupe « Programme », immédiatement au-dessus de la section préparatoire.
- **Portée programme seulement.** Un cours hors du programme n'appartient par définition à aucun bloc ; une jumelle par concentration et par profil aurait mis trois lignes indiscernables au menu d'entente.

## Alternatives rejetées

- **L'écrire dans `data/programmes/*.json`** (à côté de `add_preparatory_rules`) : exige de rescraper tous les millésimes, et oblige à répéter à l'import la garde anti-double-insertion que la préparatoire porte déjà (`import.rs::build_local_program_from_json`). Surtout, elle ferait dire au répertoire quelque chose qu'il ne dit pas.
- **Une jumelle par concentration et profil** : trois cibles identiques au menu pour un cours qui n'appartient à aucun bloc.
- **Un `Plan.out_of_program: Vec<String>` dédié** : dupliquerait `rule_grants`, sa persistance, son lien de partage et son historique annulable pour exactement la même sémantique.
- **Une contrainte pour lui donner un badge parlant** : inventerait une exigence — aucun nombre de cours hors programme n'est requis ni plafonné.
