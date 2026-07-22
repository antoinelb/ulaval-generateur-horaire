# Exigence linguistique : un champ de programme, pas une note

Date : 2026-07-21

## Contexte

Sur quatre des six pages de bac gelées, l'exigence linguistique — « réussir ANL-2020 » pour un francophone — apparaît sous deux formes de markup.

**En prose, corps d'une règle** (génie physique, industriel, mécanique) :

> Réussir le cours ANL-2020 Intermediate English II. L'étudiant qui démontre qu'il a acquis ce niveau (VEPT : 53) lors du test administré par l'École de langues peut choisir un cours d'anglais de niveau supérieur ou, s'il a acquis le niveau Advanced English II (VEPT : 63), un cours d'une autre langue moderne…

La grammaire des règles ne la reconnaît pas : elle tombe en `RuleCourses::Raw` et lève une anomalie `MalformedEntry { selector: "rule" }` — 5 lignes de `data/programmes_errors.log`, dont trois identiques dupliquées par concentration en génie mécanique.

**En deux encadrés, notes de la Règle 4** (génie des eaux) :

> Pour la personne francophone, la réussite du cours ANL-2020 Intermediate English II (VEPT : 53) est requise pour diplômer.
> Pour la personne non-francophone, la réussite du cours FLS-2093 Rédaction de textes argumentatifs (TCF-TP : 400 et TCF-TP/ÉÉ : 14) est requise pour diplômer.

L'ADR `2026-07-notes-en-prose-conservees` les rangeait dans `notes`, « affichées, jamais interprétées » — donc invisibles au solveur.

Or ce n'est pas une simple prescription en prose comme un stage.
C'est une porte de diplômation conditionnelle : **le score au test dispense du cours**.
Un francophone qui obtient VEPT ≥ 53 est exempté d'ANL-2020 ; sinon il doit le suivre.
Pour que l'interface puisse un jour sélectionner automatiquement le cours quand le seuil n'est pas atteint, le seuil doit être une donnée comparable (un nombre), pas une phrase.

## Décision

1. Nouveau champ `language_requirement: Option<LanguageRequirement>` sur `Program`, `#[serde(default, skip_serializing_if = "Option::is_none")]`.
2. Deux branches nommées comme la page les écrit :
   `LanguageRequirement { francophone: LanguageQualification, non_francophone: Option<LanguageQualification> }`.
   `francophone` est toujours présent dans les données observées ; `non_francophone` n'est explicité que par la forme à deux encadrés.
3. Chaque branche porte le cours, les seuils de test et le texte source :
   `LanguageQualification { course: String, tests: Vec<PlacementTest>, raw: String }`, `PlacementTest { name: String, score: i64 }`.
   Les seuils sont ET-liés (FLS-2093 en a deux : TCF-TP : 400 **et** TCF-TP/ÉÉ : 14).
4. Extraction bornée et déterministe, `raw` conservé intégralement :
   - `course` = premier code `[A-Z]{3}-\d{4}` de la phrase ;
   - `tests` = tous les « NOM : SCORE » du **premier** groupe parenthésé — ce qui capte le composé « (TCF-TP : 400 et TCF-TP/ÉÉ : 14) » et écarte le palier bonus « (VEPT : 63) » de la forme en prose, qui reste dans `raw`.
5. La source est déplacée, pas dupliquée :
   la règle en prose reconnue n'est plus émise (elle devient le champ, sans anomalie) ; les deux notes de la Règle 4 sont retirées de ses `notes`.
   ANL-2020 et FLS-2093 **restent** dans la liste de cours de la Règle 4 du bac en génie des eaux : ce sont de vraies options du thème « Langue et communication », qui se trouvent aussi satisfaire l'exigence.
6. Reconnu n'est pas tout parsé :
   `raw` garde la phrase entière, donc le palier VEPT 63, la voie « autre langue moderne » et la dispense par l'École de langues restent affichables même si les deux champs structurés ne les portent pas.

Conséquence assumée : retirer la règle en prose laisse un trou de numérotation (génie physique garde « Autres exigences – Règle 2 » sans « Règle 1 »).
C'est fidèle à la page — les titres ne sont jamais réindexés.

Cet ADR amende partiellement `2026-07-notes-en-prose-conservees` : l'exigence linguistique quitte `notes` pour un champ interprété ; les autres proses (stages exigés, étiquettes de sous-groupes) y restent.

## Alternatives rejetées

- **La garder en `notes`, non interprétée** (état antérieur) : un seuil de test noyé dans une phrase n'est pas comparable au score d'un étudiant ; l'interface ne pourrait jamais décider si le cours est requis.
- **Une forme plate `{course, raw}`, anglais seulement** : perd la branche non-francophone (FLS-2093) que le bac en génie des eaux énonce explicitement, avec son double seuil TCF.
- **Une liste générique de branches taguées par public** : plus flexible mais sur-générale pour deux publics connus et stables ; deux champs nommés lisent mieux et interdisent l'état vide.
- **La modéliser comme une variante de `RuleCourses`** : ce n'est pas un choix de cours à l'intérieur d'une règle mais une exigence transversale au programme ; l'y enfermer la cacherait au niveau où le solveur en a besoin.
- **Découper la phrase générale pour isoler le reste** (déjà rejeté par `2026-07-texte-brut-de-regle-paragraphe-complet`) : on ne découpe pas la prose française ; on reconnaît une forme stable et connue et on en extrait des champs bornés, exactement comme `any` et la référence de règle reconnaissent un préfixe.
