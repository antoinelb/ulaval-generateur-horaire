# Catalogue : URLs absolues telles que scrapées

Date : 2026-07-17

> **Précision** (2026-07-17, après re-gel des fixtures via `curl`) : les hrefs absolus observés étaient un artefact du « Enregistrer sous » du navigateur, qui réécrit les URLs à la sauvegarde.
> Le HTML servi par le site contient des hrefs **relatifs** (`/etudes/cours/…`).
> La décision tient — le catalogue garde des URLs absolues — mais c'est le parseur qui les construit, en préfixant `https://www.ulaval.ca` au href relatif.

## Contexte

Les pages du catalogue contiennent des URLs absolues (`https://www.ulaval.ca/etudes/cours/…`), mais le cas de test `tests/fixtures/test_cases/catalogue/gex.json` attendait des URLs relatives (`/etudes/cours/…`).
Le parseur aurait donc dû retirer le préfixe hôte — une transformation de plus, et une divergence entre la donnée source et le snapshot.

## Décision

Le champ `url` du catalogue garde l'URL **absolue, telle que scrapée**.
`gex.json` est mis à jour en conséquence ; le parseur ne transforme pas les URLs.

L'UI étant un site statique hébergé hors de `ulaval.ca`, les liens vers les pages de cours doivent de toute façon être absolus pour résoudre correctement.

## Alternatives rejetées

- **URLs relatives (état antérieur du cas de test)** : économise ~20 octets par cours, mais ajoute une étape de transformation au parseur et obligerait l'UI à re-préfixer l'hôte pour produire des liens fonctionnels.
