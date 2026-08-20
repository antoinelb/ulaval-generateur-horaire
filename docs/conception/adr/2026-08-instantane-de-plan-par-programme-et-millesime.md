# Un instantané de plan par (programme, millésime)

## Contexte

Un seul `Plan` servait tous les programmes : choisir B-GMC après B-GCI gardait la grille GCI entière sous le nom du GMC — crédits faussés, « Obligatoires 7/35 » comptant des homonymes, état incohérent survivant au rechargement, et jusqu'à une panique de rendu (rapport étudiante-cegep 2026-08-19, reproduit trois fois).
La version JS sœur garde une clé par (programme, millésime) (ADR JS `2026-08-persistance-par-programme-et-millesime-et-partage-par-fragment`) ; le user story 10 exige « la grille est vidée au passage » et « revenir redonne exactement le même panneau ».

## Décision

- `gh.v1.plan` reste le **document vivant** — y compris l'état picker (`program: None`) : la clé vivante *est* le pointeur, aucune clé « dernier ».
- `gh.v1.plan/{code}-{semester}` (même nommage que `data/programmes/`) est l'**étagère** : un instantané complet du `Plan`, même enveloppe versionnée, écrit **uniquement à la bascule, synchrone** (jamais derrière le débounce de sauvegarde) et lu uniquement à l'entrée.
- « changer » tablette le document et rend le picker ; « Choisir » restaure l'instantané exactement (sa concentration/profil priment sur les défauts du clic ; code et semester sont **forcés à la clé**, qui est l'identité) ou part d'un document frais.
- Logique pure dans `persist.rs` (`snapshot_key`, `DocumentSwap`, `leave_document`, `enter_document`, `import_stash`), testée nativement ; les composants ne font qu'écrire localStorage et les signaux (`swap_document`).
- **Migration : aucune** — rien n'est déployé ; un `gh.v1.plan` existant se restaure tel quel comme document vivant.

## Alternatives rejetées

- **Une clé map unique {clé → Plan}** : chaque sauvegarde débouncée réécrirait tous les plans, et un quota plein perdrait tout d'un coup — l'inverse du choix JS déjà arbitré.
- **Purge annulable sans étagère** : revenir à l'ancien programme ne redonnerait pas sa grille sans repasser par l'historique — le user story 10 exige la restauration exacte.
- **Écrire l'étagère au débounce** : une fermeture d'onglet entre débounce et bascule laisserait des étagères périmées ambiguës.
