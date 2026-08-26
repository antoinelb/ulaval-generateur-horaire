# Le panneau ne répète pas ce qu'il sait déjà

Date : 2026-08-26

## Contexte

Quatre éléments du panneau redisaient une information déjà portée par un voisin immédiat, ou occupaient une place sans rien dire.
Chacun coûtait une ligne, un pixel ou une phrase à lire à un étudiant qui cherche autre chose.

## Décisions

- **La barre de progrès des obligatoires disparaît.**
  Elle traduisait en pixels le badge « 1/2 » posé à trois millimètres d'elle, sur la même ligne d'en-tête.
  `Section.progress` n'avait que deux producteurs, tous deux « obligatoires » (`mandatory_section`, `scoped_mandatory_section`) : le champ part avec la barre, ses six sites d'initialisation et ses deux règles CSS.
- **Le suffixe « - en sus » disparaît du badge et de l'en-tête de règle.**
  La règle « Stages » porte déjà sa note scrapée, affichée sous elle : « Les crédits de ces stages sont en sus des crédits exigés du programme ».
  Le fait reste donc à l'écran (TRU-1), énoncé une fois et en toutes lettres plutôt que deux fois dont une en abrégé.
  `en_sus()` disparaît ; `constraint_fraction` n'a plus besoin de son `&Rule`, et `constrained()` — qui n'existait que pour lui rendre le couple `(rule, constraint)` — se réduit à `rule.and_then(|rule| rule.constraint.as_ref())` chez ses deux appelants.
- **Le verdict « Cheminement vérifié ✓ — préalables, plafond, horaires et règles comptées : tout y est. » disparaît.**
  Toutes les sections affichent alors « ✓ » et l'en-tête affiche le compte des crédits : la phrase ne faisait que les résumer.
  Le cas partiel garde ses deux lignes (« Placement vérifié ✓ » puis « ⚠ mais N sections… ») : là, le verdict dit quelque chose que les badges ne disent pas d'un coup d'œil.
- **Le badge de « Hors programme » compte ses cours** — le nombre seul, jamais sur un total : la règle n'a pas de contrainte, donc aucun dénominateur n'existerait sans l'inventer.
  Il se calcule dans `rule_section`, après `rule_badge`, à partir des `rows` déjà construites : le badge dit exactement ce que la section montre.
  Un « — » figé occupait la place sans rien apprendre — même raison que `preparatory_badge` (ADR `2026-08-regle-hors-programme` prévoyait « — » ; il est superseded sur ce point).

## Alternatives rejetées

- **Garder la barre en retirant le badge** : la barre seule ne donne ni le numérateur ni le total, et un remplissage n'est pas lisible en niveaux de gris (INP-3).
- **Compter `report.counted` plutôt que `rows`** pour « Hors programme » : le rapport et la section peuvent diverger d'un cours crédité, et c'est la section que l'étudiant a sous les yeux.
- **Réduire « tout y est » à un « ✓ » nu sur le bloc** : un état complet resterait distinct d'un état non encore vérifié, mais la vérification tourne en continu et le bloc entier serait alors un ✓ sans phrase — un ornement de plus.
