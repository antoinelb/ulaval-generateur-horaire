# La règle linguistique en prose reste dans le bloc, réécrite comblable

Date : 2026-08-28

## Contexte

`take_language_rules` (ADR `2026-07-exigence-linguistique-champ-dedie`) retire entièrement la règle dont le corps est reconnu comme prose de l'exigence linguistique — génie physique, industriel, mécanique — une fois son contenu recopié dans `program.language_requirement`.
Rapport du 2026-08-27 sur B-GMC-A26 : chaque concentration perd ainsi sa « Règle 2 – 3 crédits : Réussir le cours ANL-2020 », donc n'affiche plus que 15 cr de règles face à un `credits_required` de 18.
La jauge devient structurellement infaisable — aucune sélection de cours ne peut jamais l'atteindre — et le total du programme plafonne à 117/120 crédits.
Le même retrait touche B-GIN et B-GPH.

## Décision

- `take_language_rules` ne supprime plus la règle : elle la réécrit en place.
  Le champ `language_requirement` est rempli comme avant (premier-gagnant conservé entre les occurrences répétées par concentration), mais le corps en prose est déplacé dans `rule.notes` — il reste affichable, dispense VEPT, palier 63, autre langue moderne — et `rule.courses` devient `RuleCourses::List { courses: vec![code] }`, `code` étant le premier sigle extrait de la prose par `first_course_code`, la même extraction que `LanguageQualification::course`.
- La contrainte de la règle (`Constraint::Credits { min: 3, max: 3 }`, déjà lue dans son titre) est inchangée : la règle redevient comblable, et une entente négociée peut désormais la couvrir puisque les grants exigent une `List`.
- Aucun sigle trouvé dans la prose → la règle reste `Raw`, inchangée : on n'invente jamais un code.
- `take_language_notes` (forme « deux encadrés » de génie des eaux) n'est pas touchée : elle ne retirait déjà que des notes, jamais de crédits.
- Fixtures régénérées : `B-GMC.json`, `B-GIN.json`, `B-GPH.json` — chacune récupère sa règle linguistique à sa position d'origine.
  `B-GCI.json`, `B-GEX.json`, `M-GEX.json` sont inchangées (forme deux-encadrés ou pas de règle de langue).
- Millésimes antérieurs (A22 à H27) laissés **intacts** : ils continuent de totaliser sans cette règle, seul le millésime A26 est re-scrapé par ce lot.

## Effet de bord voulu

ANL-2020 devient un cours ordinaire d'une règle `List`, donc visible et plaçable par le solveur d'organigramme — auparavant la prose retirée ne référençait aucun cours nulle part dans les règles.

## Alternatives rejetées

- **Ajuster `credits_required` pour retirer les 3 crédits manquants** — le chiffre affiché ne correspondrait plus à celui de la page ULaval ; le programme prétendrait exiger moins que ce que la source dit.
- **Supprimer `language_requirement` et ne garder que la règle réécrite** — perdrait le signal dédié (seuils de test structurés, dispense VEPT) que l'UI et un futur solveur de préférences lisent directement sans reparser une note.
- **Laisser la règle sans contrainte, comptée à 0 crédit** — la jauge resterait fausse : la règle existerait mais ne compterait pour rien, ce qui masque le même trou différemment.
