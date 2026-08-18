# Le parseur de préalables déménage du scraper vers core

Date : 2026-08-17

## Contexte

`parse_prereq_tree` vivait dans `crates/scraper/src/parser/course.rs`, au milieu des sélecteurs CSS.
Or ni `ui` ni `wasm` ne dépend de `scraper` : la grammaire des préalables était inatteignable depuis l'application.
Laisser un étudiant réécrire un préalable dans la langue du répertoire (ADR `2026-08-correction-des-prealables-par-millesime`) l'exigeait.

C'est aussi une correction de couche : la grammaire d'un concept du domaine est de la logique métier, et l'invariant du projet la veut dans `core`.

## Décisions

- `crates/core/src/prereq_parse.rs` reçoit `parse_prereq_tree`, son tokenizer et ses replieurs (`classify_operand`, `checkable_operand`, `fold_frame`, `fold_chain`, les bornes de crédits), avec leurs tests.
- L'erreur devient `PrereqParseError` de `core` ; le scraper l'enveloppe dans son `ParseError::MalformedPrerequisites`, dont la forme JSON ne change pas.
- `is_course_code` devient public : `parse_equivalents` valide avec lui le contenu des cartes d'équivalence.
- **`parse_preuniversitaire` reste dans le scraper**, avec `normalize_connectors`, `contains_sigle` et `leading_course_code`.
  La frontière n'est pas « tout ce qui touche aux préalables » mais « ce qui lit du texte de grammaire » contre « ce qui lit du HTML ».

## Alternative rejetée

- **Faire dépendre `ui` de `scraper`** : une crate binaire native async (`reqwest`, `tokio`, `scraper`) tirée dans un bundle wasm pour une fonction pure de 250 lignes.
