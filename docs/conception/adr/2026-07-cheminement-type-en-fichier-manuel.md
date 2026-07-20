# `cheminement_type` dans un fichier manuel

Date : 2026-07-20

## Contexte

Le `cheminement_type` (l'organigramme A1→H8) n'a aucune source machine-lisible : il est encodé à la main, pour le bac en génie des eaux seulement (`docs/project_plan.md` § Source de données).

`CLAUDE.md` le décrivait comme faisant partie de `data/programmes.json`.
Or `core::Program` n'a pas ce champ, et le scraper ne peut pas le produire : dès qu'il écrit le fichier d'un programme, il effacerait une donnée saisie à la main.

Le même problème s'est déjà posé pour les cours, et a déjà une réponse : `data/cours/{session}.manuel.json`, maintenu à la main, que le scraper ne touche jamais (ADR `2026-07-contribution-de-cours-manuels`).

## Décision

1. Le `cheminement_type` vit dans `data/programmes/{code}.manuel.json`, à côté du fichier scrapé et jamais écrit par le scraper.
   Comme un run n'écrit que `{code}.json` pour les URL qu'on lui a nommées et ne balaie rien (ADR `2026-07-un-fichier-par-programme`), aucun code de garde n'est nécessaire : le fichier manuel est hors de portée par construction.
2. `data/programmes/{code}.json` appartient donc entièrement au scraper, ce qui rend son remplacement atomique sûr.
3. La lecture des deux fichiers et leur composition sont l'affaire de `core`, au jalon du cheminement type — pas du scraper.
4. `CLAUDE.md` et `docs/project_plan.md` sont corrigés : ils décrivaient un fichier unique contenant les deux.

## Alternatives rejetées

- **Un champ optionnel `cheminement_type` sur `core::Program`** : obligerait le scraper à relire le fichier existant et à en reporter la valeur à chaque run, donc à dépendre d'une donnée qu'il ne produit pas — et un fichier absent, tronqué ou illisible ferait silencieusement perdre l'encodage manuel.
- **Un fichier global `data/cheminements.json`** : sépare l'organigramme de son programme sans rien simplifier, et rouvre la question du remplacement en bloc que l'ADR `2026-07-un-fichier-par-programme` vient de fermer.
