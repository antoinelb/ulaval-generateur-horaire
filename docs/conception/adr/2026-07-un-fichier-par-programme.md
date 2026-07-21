# Un fichier par programme

Date : 2026-07-20

## Contexte

`docs/project_plan.md` annonçait un `data/programmes.json` unique, à côté de `data/catalogue.json` et de `data/cours/{session}.json`.
En écrivant la commande `ulaval-scraper program`, deux propriétés de cette commande ont rendu le fichier unique inconfortable.

D'abord, la commande n'a **aucune file de travail dérivable** : l'URL d'une page programme est un slug (`baccalaureat-en-genie-civil`) qu'aucun code ne reconstruit, et seuls les programmes dont on veut les règles ont besoin d'être scrapés.
Les URL sont donc des arguments obligatoires, et chaque run porte sur un sous-ensemble choisi à la main.

Ensuite, un fichier unique force à trancher entre deux comportements également inconfortables : le réécrire en bloc (un run à une seule URL effacerait tous les autres programmes) ou y fusionner par code (l'état du fichier devient un cumul d'exécutions successives, irreproductible depuis une seule commande, et un programme retiré du bac n'en disparaît jamais).

## Décision

1. Un fichier par programme : `data/programmes/{code}.json`, où `code` est le slug d'URL — la clé que `core::Program` porte déjà.
   Le fichier est un `core::Program` sérialisé **directement**, sans enveloppe : c'est exactement la forme des fixtures `tests/fixtures/test_cases/programs/*.json`, qui deviennent donc des exemples de sortie de production, et que le test d'intégration compare octet pour octet à ce que la commande écrit.
2. Un run écrit exactement les programmes qu'on lui a nommés et ne touche à rien d'autre.
   **Aucun balayage** des fichiers périmés : la commande est toujours restreinte, comme un `courses --subjects`, qui laisse déjà les autres snapshots tranquilles (ADR `2026-07-nettoyage-des-snapshots-perimes`). Elle n'a jamais vu la liste complète des programmes et ne peut donc juger aucun fichier obsolète.
3. Écriture atomique par fichier (`write_atomic`), comme les autres artefacts, et un `data/programmes_errors.log` unique à côté du répertoire — même patron que `cours_errors.log`.
4. Pas de cache. Celui des cours se justifie par un run de ~17 min sur ~10 000 pages (ADR `2026-07-cache-de-cours-parses`) ; ici il y a une poignée d'URL et le run dure quelques secondes.

## Alternatives rejetées

- **Un `programmes.json` réécrit en bloc** : la liste d'URL serait l'énoncé complet du snapshot, ce qui est prévisible — mais un run à une URL effacerait les autres programmes, et le `cheminement_type` encodé à la main avec.
- **Un `programmes.json` fusionné par code** : plus commode pour ajouter un programme, mais le contenu du fichier ne dépend alors plus de la commande, seulement de l'historique des runs ; impossible à reproduire, et rien n'en sort jamais.
- **Un balayage des fichiers non produits** : demanderait à la commande de connaître la liste complète des programmes, ce qui est précisément ce qu'elle n'a pas.
