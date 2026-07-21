# Une exigence de crédits peut ne nommer aucun programme

Date : 2026-07-19

## Contexte

La grammaire des préalables ne connaissait qu'une forme d'exigence de crédits, `PROGRAMME, Crédits exigés : N`, reconnue par le mot précédant la virgule.
GEX-3333 écrit la sienne sans programme du tout :

```
(ECN-2901 OU ECN-4901) ET GCI-1011 ET GMC-3009 ET  Crédits exigés : 72
```

— la double espace marquant la place vide.
Faute de virgule, `Crédits` était traité comme un sigle de cours, échouait, et **tout** le préalable basculait en `Prerequisites::Raw` : les trois cours parfaitement lisibles de l'expression étaient perdus pour le solveur avec lui.

L'exigence porte alors sur le programme de l'étudiant lui-même, quel qu'il soit.

## Décision

- `core::ProgramCredits.program` devient `Option<String>`, `#[serde(default)]`, sérialisé `null` quand il est absent — même convention que `Course.prerequisites` (ECN-4901).
- Le tokenizer reconnaît la forme nue : le mot `Crédits` suivi de `exigés : N` produit `Credits { program: None, credits }`.
  Un `Crédits` qui n'est pas suivi de cette séquence reste hors grammaire (`credits requirement`), donc signalé.
- GEX-3333 rejoint les fixtures gelées.

## Alternatives rejetées

- **Substituer le sigle du cours** (GEX-3333 → programme « GEX ») : une invention.
  Rien sur la page ne dit que l'exigence porte sur GEX plutôt que sur le programme de l'étudiant, et le sigle d'un cours n'est pas celui d'un programme.
- **Chaîne vide plutôt qu'`Option`** : rend « aucun programme » indistinguable d'un programme mal lu, et oblige chaque consommateur à connaître la convention.
- **Laisser l'expression en brut** : c'est le comportement qu'on corrige — une exigence non reconnue faisait perdre les trois cours corrects de la même expression.
