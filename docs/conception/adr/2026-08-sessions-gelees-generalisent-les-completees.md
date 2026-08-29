# Les sessions gelées généralisent les sessions complétées

Date : 2026-08-29

## Contexte

L'évaluation persona du 2026-08-29 (directeur B-GCI) a montré qu'aucun geste naturel n'exprime « ces cours sont faits tels quels, n'y touche plus » sans relevé Capsule : le seul mécanisme fermant des sessions au solveur était `completed_sessions`, un préfixe `1..=n` alimenté exclusivement par l'import de relevé (ADR `2026-08-sessions-completees-fermees-au-solveur`).
Le scénario le plus demandé — « cet étudiant a coulé tel cours, qu'est-ce que ça change ? » — exigeait de figer le passé vécu puis de laisser le solveur réarranger le futur seul.

L'ADR `2026-08-retrait-de-la-notion-de-cours-reussi` avait rejeté les « sessions passées à bascule manuelle » — mais pour *marquer le passé*, que le calendrier donne déjà. Geler est autre chose : une contrainte de solveur (« cette session est réglée »), que ni le calendrier ni le relevé ne connaissent.

## Décision

`completed_sessions: usize` (préfixe) devient `frozen: BTreeSet<usize>` (indices 1-based, ensemble quelconque) dans `Plan`, `OrganigrammeInput` et `PlacementRequest`. Une session gelée :

- ne reçoit aucun cours non épinglé (même filtre de domaine qu'avant, l'appartenance remplaçant la comparaison au préfixe) ;
- ne laisse pas partir ce qui s'y affiche : `with_request` (wasm) épingle chaque entrée de la graine assise dans une session gelée — connue du catalogue de la requête, ni déjà épinglée ni réussie — avant d'appeler le solveur. Le « ne rien déplacer » se réalise donc à la construction de la requête, `core` ne connaissant que la fermeture de domaine ;
- reste une session comme une autre pour l'utilisateur : glisser dedans, dehors, retirer — tous gestes souverains et annulables. Sortir un cours d'une session gelée (un échec à reprendre) le rend flottant et le solveur réarrange le futur sans toucher au gelé.

En surface : une bascule « Geler cette session » dans l'en-tête de l'horaire (un acte `edit_plan`, ACT-2), un insigne « ❄ gelée » sur la carte du ruban (glyphe + mot, jamais couleur seule, INP-3). L'import Capsule gèle les sessions notées (`plan.frozen.extend(1..=n)` — extension, jamais écrasement : un gel posé par l'étudiant sur une session future survit au rechargement du relevé). Le fichier cheminement gagne un `"frozen": true` optionnel par session (absent = faux, les grilles officielles et les vieux fichiers restent valides) ; une session gelée hors de l'horizon est nommée au bilan, jamais tue.

Le partage par URL ne porte pas le gel — même arbitrage que l'ADR des sessions complétées (commentaire `ponytail:` déjà posé dans `persist.rs`).

## Alternatives rejetées

- **Marques « réussi »/« échoué » par cours** : redondantes — placé dans une session gelée = fait ; retiré = à refaire. Déjà tranché une fois par l'ADR du retrait.
- **Garder `completed_sessions` à côté de `frozen`** : deux sources de vérité sur « fermé au solveur », le préfixe n'étant qu'un cas particulier de l'ensemble.
- **Ancrer dans `core` (épingler la graine gelée dans `place`)** : mêlerait la graine (une préférence) à l'épinglage (une contrainte) au cœur de la recherche ; la construction de requête est l'endroit qui voit déjà les deux.
- **Matérialiser le gel en épinglages dans `Plan.pinned_sessions`** : dégeler devrait alors distinguer ses épingles de celles de l'étudiant — un état de plus à porter pour rien.
