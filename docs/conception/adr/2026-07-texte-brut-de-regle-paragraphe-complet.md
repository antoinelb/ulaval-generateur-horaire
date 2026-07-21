# `raw` d'une règle : le paragraphe complet

Date : 2026-07-20

## Contexte

Quand une règle n'énumère pas de cartes de cours, son contenu est une phrase que la grammaire essaie de reconnaître : « tous les cours de premier cycle… » → `Keyword::Any`, « tous les cours de la Règle N du cheminement X » → `RuleReference`.
Le texte source est conservé dans `raw` (ADR `2026-07-reference-de-regle-structuree`).

Le bac en génie civil écrit, dans **un seul** `p.fe-bloc-regle--ligne` :

> tous les cours de premier cycle, à l'exception des cours correctifs de français. Si le profil développement durable fait partie de votre cheminement, vous devez suivre DDU-1000.

L'ancienne fixture ne gardait que la première phrase.
La prescription DDU-1000 disparaissait — silencieusement.

## Décision

1. `raw` reçoit **le paragraphe entier**, tel que la page l'écrit.
   La grammaire en reconnaît un **préfixe** ; elle ne réécrit ni ne tronque le texte.
2. Le paragraphe est normalisé par compression des blancs uniquement (`split_whitespace`), ce qui absorbe les retours ligne, les tabulations et les `&nbsp;` du source.
   Les apostrophes typographiques (U+2019) et la ponctuation finale sont conservées telles quelles : `raw` est du texte destiné à être affiché, pas une clé.
3. Quand le corps d'une règle compte plusieurs paragraphes, le premier devient `raw` et les suivants deviennent des `notes` (ADR `2026-07-notes-en-prose-conservees`).

## Alternatives rejetées

- **Ne garder que la phrase reconnue** (forme des fixtures d'origine) : perd la prescription qui suit, et rend le `raw` non vérifiable contre la page — le lecteur ne peut plus comparer.
- **Découper le paragraphe en phrases et ne signaler que le reste** : demande un découpage de phrases en français (abréviations, « VEPT : 53 ») pour un gain nul, la grammaire n'ayant rien à faire du reste.
