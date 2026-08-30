# Un dépassement de règle est un statut rouge sur cette règle, jamais une panne du panneau

Date : 2026-08-30

**Statut :** accepté (arbitrage d'Antoine). **Remplace** `2026-07-somme-au-dessus-du-max-en-erreur-typee`, dont il est l'arbitrage annoncé. **Amende** `2026-08-verdicts-honnetes-et-panneau-jamais-vide`, `2026-08-erreur-de-comptage-nommee-par-portee` et `2026-08-contrainte-etiquetee-min-max`.

## Contexte

Sélectionner un cours de trop dans une règle — 15 crédits là où elle en admet 12, deux cours dans une règle « Un cours parmi » — effaçait **tout le panneau des exigences** : chaque portée retombait sur « —/18 cr — progression indisponible », chaque insigne sur « — ».

La cause était une granularité, pas une intention.
`verdict` renvoyait `Err(CreditsOverMax)`/`Err(CountOverMax)`, que `coverage_report` propageait par un `?` sur un `collect::<Result<Vec<Vec<RuleReport>>, _>>()` : la première règle en défaut, dans n'importe quelle portée, annulait le rapport entier.
Trois indices disaient que le comportement avait dépassé la décision :

- le message affiché promettait déjà « Cette règle seulement : les autres continuent d'être comptées », ce que le code ne faisait pas ;
- le commentaire justifiant le repli `uncounted_panel` invoquait **AIR ERR-5** (« *one failed data source degrades one region, marked* »), la règle même qu'il enfreignait ;
- l'ADR `2026-07-somme-au-dessus-du-max-en-erreur-typee` annonçait « à l'arbitrage, l'erreur sera remplacée par la sémantique décidée ».

## Décision

**Le dépassement est une violation, montrée en rouge sur sa seule règle.** C'est l'arbitrage que l'ADR de juillet attendait : ni « satisfaite », ni excédent silencieux.

- `RuleStatus` gagne `OverMax` et `Uncounted` (sérialisés `over_max`/`uncounted` — l'enum passe de `lowercase` à `snake_case`, sans effet sur les trois variantes existantes).
- `RuleReport` gagne `defect: Option<RuleDefect>`, avec deux variantes : `MissingCourse { code }` et `BrokenReference { concentration, target }`. Les trois échecs de chasse de référence n'en font qu'une : ils se lisent pareil pour qui regarde la règle.
- `verdict` ne renvoie plus de `Result` : il ne peut plus échouer.
- `CoverageError` **passe de huit variantes à deux** : `UnknownConcentration` et `UnknownProfile`, les seules qui ne visent aucune règle et ne laissent aucune portée à rapporter. Elles seules déclenchent encore `uncounted_panel` — ERR-5 à la bonne granularité.
- `counted` garde tous les codes en dépassement : c'est lui qui porte le « 15 » de « 15/12 cr ».

Côté interface, **rien de neuf n'a été dessiné** — le rendu voulu existait déjà en aval du `Err` :

- `constraint_fraction` ne borne pas son numérateur : « 15/12 cr » sort tel quel ;
- `Badge::Missing` pilote `.panel-rule--missing`, donc bordure `--accent` et fond d'en-tête `--accent-bg` ;
- `rule_lead` affiche déjà une explication sous une règle en défaut ; le dépassement y pose le conseil qui vivait dans le bandeau d'erreur (« Retirez-en un, ou rattachez-le à une autre règle avec le menu « entente avec la direction… » »).

Effet en cascade : le `.transpose()?` de `wasm::organigramme::verify` ne pouvait plus échouer — `intake` valide déjà la concentration et le profil avec les deux mêmes refus. Un organigramme dont une règle est trop remplie rend désormais son verdict au lieu d'être jeté.

La référence `tests/reference/solveur_b/verify_rules.py`, versionnée jusqu'à l'arbitrage (`2026-07-reference-b-versionnee-jusqua-larbitrage`), porte la même sémantique ; deux fixtures la gèlent : `gex-rule4-credits-over-max` et `gex-rule4-credits-uncounted`.

## Alternatives rejetées

- **Garder l'erreur typée et n'élargir que le message** : le panneau resterait vide, c'est-à-dire le défaut signalé.
- **Compter le dépassement comme `satisfied` avec excédent ignoré** : rend la faute invisible, alors que la demande est précisément de la rendre évidente.
- **Quatre variantes de `RuleDefect`** (une par échec de référence) : trois d'entre elles produisent la même phrase à l'écran ; les distinguer n'aiderait personne.
- **Dégrader aussi les deux erreurs de portée par règle** : une concentration inconnue ne désigne aucune règle — il n'y a rien à marquer, seulement une portée à ne pas afficher.
