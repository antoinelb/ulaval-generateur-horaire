# La règle linguistique en prose élargie aux cours que sa phrase autorise

Date : 2026-08-29

Amende `2026-08-regle-linguistique-conservee-comblable`, qui amendait déjà
`2026-07-exigence-linguistique-champ-dedie`.

## Contexte

`take_language_rules` réécrit la règle en prose de l'exigence linguistique en
`RuleCourses::List { courses: vec![first_course_code(&raw)] }` — un seul sigle,
toujours `ANL-2020`.
L'interface affiche donc « Règle 2 ✓ 3/3 cr » avec **une carte verrouillée**,
alors que la phrase juste au-dessus offre explicitement un choix :

> Réussir le cours ANL-2020 Intermediate English II.
> L'étudiant qui démontre qu'il a acquis ce niveau (VEPT : 53) lors du test administré par l'École de langues **peut choisir un cours d'anglais de niveau supérieur** ou, s'il a acquis le niveau Advanced English II (VEPT : 63), **un cours d'une autre langue moderne**.

Le millésime A24 du bac en génie des eaux publie la même exigence comme une
liste de 11 options choisissables (Règle 6). Le génie mécanique n'en publie
aucune : la liste est dans la phrase, et le parseur ne voit qu'une page.

Relevé du 2026-08-29 sur 26 programmes du répertoire : **13 formulations
vivantes** de la même règle (famille A), plus deux familles distinctes
(déclaratif « est requise pour diplômer » avec liste publiée ; niveau seul sans
sigle). `peut` / `doit` / impératif nu (« minimalement… choisir ») / voix
passive (« peut être suivi ») sont quatre modalités sur un **contenu
identique** : aucune ne change l'ensemble des cours admissibles.

## Décision

### L'ensemble admissible se lit dans la phrase, se résout dans le catalogue

Nouveau module `core::language`, **hors de la feature `parser`** (aucun HTML en
jeu, et `wasm` compile `core` sans elle). `widen_language_rules(program,
courses)` réécrit la liste de chaque règle en prose comme l'union de :

1. **tout sigle que la phrase nomme elle-même** (`course_codes`) — ce qui
   couvre les trois cours d'entrée du génie chimique (ANL-2020 ou 3010 ou 3020)
   et la liste fermée du génie agroenvironnemental (`EDC-1001`…) sans code
   dédié ;
2. **l'anglais** : si la phrase nomme au moins un `ANL-NNNN`, tous les `ANL` de
   premier cycle du catalogue **au-dessus du plus bas sigle qu'elle nomme**. Le
   plancher est *lu*, jamais présumé — le génie écrit ANL-2020, l'anthropologie
   et la psychologie ANL-3010 ;
3. **la langue moderne** : si la phrase contient « langue moderne », tous les
   cours de premier cycle des sigles `MODERN_LANGUAGE_SUBJECTS`.

Aucune modalité n'est analysée : une sous-chaîne et une liste de sigles
couvrent onze des treize variantes relevées, ce qui évite le découpage de prose
française que `2026-07-texte-brut-de-regle-paragraphe-complet` interdit.

### Les huit sigles viennent de ULaval, pas de nous

```rust
pub const MODERN_LANGUAGE_SUBJECTS: &[&str] =
    &["ALL", "ARA", "CHN", "ESG", "ITL", "JAP", "POR", "RUS"];
```

Recopiés de la note de la Règle 3 du bac en anthropologie
(`data/programmes/B-ANT-A26.json`) :

> « et les cours de langue moderne portant les sigles **ALL, ARA, CHN, ESG, ITL, JAP, POR et RUS**. »

`ESP` (études hispaniques) en est **absent** : `ESG` est l'espagnol de l'École
de langues, celui que la page nomme. `FLS` et `FRN` aussi : ce sont les
branches non-francophones, portées par `Program.language_requirement`, pas une
« autre langue moderne » pour l'étudiant que cette règle vise.

Résultat pour le B-GMC/B-GIN/B-GPH : **78 cours** (6 ANL ≥ 2020, 71 de langue
moderne, plus le gabarit ci-dessous), tous à 3 crédits, tous avec horaire
publié.

### L'expansion vit hors du parseur, comme la scolarité préparatoire

Même contrainte, même patron que `preparatory_rule` : fonction pure de `core`
prenant le catalogue en paramètre, appelée par le scraper
(`cli.rs::widen_language_rules_of`, à côté de `add_preparatory_rules`) et
rejouée à l'identique par l'import UI (`import.rs::build_local_program`) — un
programme importé ne doit jamais offrir un choix plus étroit qu'un programme
livré.

### Le gabarit `LAN-GUES` reste dans la liste

`data/cours.manuel.json` définit « LAN-GUES — Cours de langue selon le résultat
VEPT », 3 crédits, même famille que `OPT-ION1` et `AUC-HOIX`
(`2026-08-cours-manuels-offerts-en-toute-saison`). **Tous** les
`cheminement_type` du B-GMC et du B-GIN le placent en session comme créneau de
langue. Une règle linguistique qui ne le liste pas laisse ce placement compté
par aucune règle — exactement le symptôme que `2026-08-entente-vers-une-regle-any`
décrit pour `AUC-HOIX`. Il est donc ajouté à toute règle élargie.

### Les millésimes antérieurs sont réparés hors ligne

`2026-08-regle-linguistique-conservee-comblable` avait laissé A22–H27 intacts.
Ils ne peuvent pas être re-scrapés : leurs pages ont disparu de ulaval.ca, et
`write_programs` écrit sous le millésime **du jour** — un rafraîchissement
écraserait un instantané gelé par le programme d'aujourd'hui.

Leur règle linguistique ne contient que `LAN-GUES`, sans prose : l'extraction
d'avant le 2026-08-28 déplaçait la phrase entière dans
`language_requirement.francophone.raw`, où elle est **intacte**.

Nouveau bras `ulaval-scraper reparse --programs` : `restore_stripped_language_prose`
remet la note d'où elle vient, puis `widen_language_rules` élargit. Aucun
réseau, exactement comme `reparse` re-dérive les arbres de préalables du `raw`
déjà stocké. Un fichier inchangé n'est pas réécrit.

Le discriminant est **le gabarit nommé, pas la forme du sigle** : le
« Profil entrepreneurial – Règle 3 » du B-GMC porte `OPT-GMC1` sous la même
contrainte `credits 3..3` et sans note. Seul `courses == ["LAN-GUES"]` distingue
les deux.

### Garde-fous

- Une règle listant **plus d'un** cours n'est jamais touchée : ULaval a publié
  sa liste (`B-GEX`, `B-GCI`, `B-ANT`), aucune inférence ne la remplace. Cela
  rend l'élargissement **idempotent**.
- L'ensemble est amorcé avec ce que la règle listait déjà : il ne peut que
  croître, jamais perdre un cours.
- Un sigle nommé par la page mais absent du catalogue est **conservé**.
- Au-delà de `MAX_LANGUAGE_COURSES = 200` : `LanguageError::TooManyCourses`
  nommant la règle — jamais de troncature silencieuse.
- `is_language_prose` accepte désormais « École **des** langues » (génie
  géologique). Il reste partagé avec le parseur, qui parque ces règles en `Raw`
  sans anomalie précisément pour que cette passe les retrouve.

### Affichage

La section de règle gagne deux `<details>` natifs : « Anglais — 6 cours »
ouvert, « Autre langue moderne — 71 cours » replié. Le cours que l'exigence
nomme porte « - exigé par défaut » **dans son texte**, jamais par une position
ou une couleur (AIR INP-3). La ligne « Exigence linguistique - ANL-2020 ou
VEPT ≥ 53 » est répétée en tête de la règle : le panneau défile en interne, son
pied est presque toujours hors écran. Le filtre local existant
(`RULE_FILTER_THRESHOLD = 8`) traverse les deux groupes.

Aucun résumé ne cite le palier VEPT 63 : il vit dans la prose affichée
au-dessus et dans aucun champ analysé — l'écrire serait afficher un nombre que
personne n'a lu (AIR TRU-1).

## Hors portée, assumé

- **Les branches d'échappement ouvertes** ne sont pas énumérables : « tout autre
  cours jugé pertinent par la direction » (génie physique), « tout autre cours
  de 3 crédits » (génie logiciel, informatique), le catalogue négatif du bac en
  sciences et technologie des aliments, « à déterminer avec la direction »
  (génie chimique). Elles restent lisibles dans la note affichée verbatim, et le
  sélecteur « entente avec la direction » les couvre. On n'invente pas une liste
  pour une prose qui refuse d'en avoir une.
- **Les renvois inter-règles** — « un cours à option supplémentaire parmi ceux de
  la règle 2 ci-dessous » (génie des mines), « de la règle 1 ci-dessus » (génie
  géologique) — ne sont pas résolubles depuis la phrase seule.
- **Les familles B et C** (déclaratif avec liste publiée ; niveau seul sans
  sigle, comme le bac en droit) ne sont pas touchées : la première publie déjà sa
  liste, la seconde ne nomme aucun cours.

## Alternatives rejetées

- **Une liste figée de sigles ANL dans le parseur** : dix lignes, aucun nouveau
  point d'appel — mais n'offre pas « une autre langue moderne », que la phrase
  accorde explicitement.
- **Recopier la liste publiée par le B-GEX/B-GCI** (5 ANL + EDC/FLS/FRN/PHI) :
  cette liste vient d'une *autre page* que celle du B-GMC ; elle exclut les
  langues modernes que la prose du B-GMC autorise et inclut des cours de
  communication qu'elle n'autorise pas.
- **Une variante `RuleCourses::Subjects { subjects, floor }`** résolue au moment
  du rapport : JSON court et jamais périmé, mais touche `RuleCourses` (serde
  `untagged`), le vérificateur de couverture, la référence Python
  `verify_rules.py` à parité bit-à-bit, `wasm`, l'UI et les ententes — un
  chantier pour un gain de propreté.
- **Encoder la liste à la main dans `*.manuel.json`** : un fichier par
  millésime, alors que la prose est machine-lisible.
- **Un parcours restreint (recherche bornée aux matières de langue) au lieu
  d'une liste** : suppose la variante ci-dessus pour que `core` sache compter ce
  qui y est pris.
- **S'appuyer sur le sélecteur « entente avec la direction » déjà présent sur
  chaque carte** : l'entente décrit un accord réel avec la direction ; ici le
  choix est de droit. Présenter un droit comme une faveur, c'est mentir sur ce
  que la page dit (AIR TRU-1).
