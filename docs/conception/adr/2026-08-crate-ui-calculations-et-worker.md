# `crates/ui-calculations` : le solveur B dans un Web Worker, hors du fil principal

> **Amendé le 2026-08-17** : le crate a fusionné dans `crates/wasm` (ADR `2026-08-fusion-des-crates-wasm-et-ui-calculations`).
> Le protocole du worker et ses deux exports (`init_snapshot`, `handle_message`) restent tels quels — ils vivent désormais dans le crate de frontière, aux côtés de la surface JavaScript.
> L'alternative « réutiliser le pkg de `crates/wasm` » rejetée plus bas a été retenue depuis, son objection (le snapshot dans chaque appel) étant devenue le problème de l'interface JavaScript elle-même.

## Contexte

Les règles d'interface (AIR LAT-3, `docs/ux/interface-rules.md`) interdisent de bloquer le fil principal plus de 16 ms.
Or `place()` est un appel synchrone opaque : ~30 ms pour la première solution du bac GEX complet, et jusqu'à plusieurs secondes quand l'étudiant demande « chercher plus longtemps ».
Le harnais `ui-debug` assumait le gel de l'onglet ; la vraie UI ne le peut pas.

## Décision

Un nouveau crate `crates/ui-calculations` (lib + cdylib), décidé avec Antoine (2026-08-13), deux rôles :

- **fonctions pures partagées** consommées nativement par `ui` et testées à 100 % : `credits::credit_summary` (le « 96/120 » — crédits *en sus* et scolarité préparatoire comptés à part, réglant le point de `docs/next_steps.md` sur `credits_in_addition` ; pas dans `core` car le solveur n'en a pas besoin), `merge::merge_manual` (fusion des cours manuels, le scrapé prime, collisions remontées) ;
- **module wasm du worker** (`make ui-calc` → `crates/ui/assets/calc/`, gitignoré) : le shim `crates/ui/assets/worker.js` (~40 lignes de câblage, zéro calcul) télécharge le snapshot une fois (cache HTTP — jamais 8,6 Mo par postMessage), l'initialise avec les cours manuels, puis relaie des chaînes JSON : `protocol::Request` (`place` / `verify` / `admissible-sessions`, chacune sous un `id`) → `protocol::Response`. Une requête illisible répond sous l'id réservé 0 — jamais de silence.

Le protocole reflète l'orchestration du module JS (`wasm::organigramme`) — mêmes deux moitiés pour `verify`, mêmes règles d'étés — moins le champ `courses` (l'état du worker) et avec des budgets explicites : l'UI décide toujours.

## Alternatives rejetées

- **`place()` sur le fil principal + waiver GOV-2** : proposé, refusé par Antoine — LAT-3 à la lettre.
- **Réutiliser le pkg de `crates/wasm` dans le worker** : la frontière existe déjà, mais ses quatre fonctions prennent le snapshot *dans chaque appel* (8,6 Mo par message) et l'ADR `2026-08-module-wasm-quatre-fonctions-js` fait de `ui` et `wasm` deux consommateurs parallèles de `core` — y accrocher le worker aurait couplé les deux.
- **Une API de recherche interruptible dans `core`** : le découpage en tranches coopératives aurait imposé sa forme au solveur entier pour un besoin purement navigateur.
