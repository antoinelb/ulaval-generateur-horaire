# Le contrat de l'horaire hebdomadaire rendu à l'UI

Date : 2026-07-28

## Contexte

La v0 affiche un horaire auto-construit dès l'entrée des codes de cours, surligne les conflits évidents et laisse changer de section.
L'UI appellera donc une fonction pure de `core` à chaque ajout ou retrait de cours et à chaque changement de section — et comme toute logique métier vit dans `core`, c'est cette fonction qui doit dire quelle section est sélectionnée, si l'horaire tient, et quelles alternatives restent possibles.
Le solveur A (`weekly.rs`) n'est pas encore implémenté ; les cas de test sont écrits d'abord (`tests/fixtures/test_cases/schedules/*.json`) et fixent le contrat avant le code.

## Décision

**Entrée** : la liste des cours (leurs `options` par saison, format `core::Course`), la saison, et optionnellement `chosen` — une option épinglée par l'étudiant pour certains cours, identifiée par son ensemble de NRC trié (une option n'a pas d'identifiant propre).

**Sortie** : un objet avec `valid` (toujours présent) et `courses` dans l'ordre d'entrée, où chaque cours porte :

- `selected` : l'option retenue, sections embarquées en entier (`nrc`, `section`, `mode`, `slots`) — la sortie est autoportante, l'UI n'a rien à recouper dans le snapshot ;
- `valid: false`, champ *optionnel*, présent uniquement quand la sélection de ce cours chevauche celle d'un autre cours — le conflit est marqué dans le cours même, c'est ce que l'UI surligne ;
- `alternatives` : les options non sélectionnées dans l'ordre du snapshot, chacune avec ses `sections` complètes et le même `valid: false` optionnel en **sémantique swap** — l'alternative est invalide ssi elle chevauche la sélection courante d'un *autre* cours, les autres cours ne bougeant pas.

**Règle de sélection déterministe « premier horaire faisable »** : cours dans l'ordre d'entrée, options dans l'ordre du snapshot ; première combinaison complète sans conflit qui respecte les épinglés.
S'il n'en existe aucune : `valid: false`, les épinglés sont gardés (le choix explicite de l'étudiant n'est jamais défait), les autres cours prennent leur première option, et les cours en chevauchement sont marqués.
Le classement par préférences du jalon 10 remplacera cette règle ; le reste du contrat n'en dépend pas.

**Fixtures** : un fichier par phénomène (18 cas — relations de plages dont dos-à-dos et conflit sur la 2ᵉ plage seulement, modes à plages vides, sélection qui saute une option en conflit, divergence swap/global, infaisable par paires, infaisable à trois cours aux paires compatibles, NRC partagé, labo seul en conflit, épinglages), cours embarqués sous codes réels mais plages ajustées pour produire le phénomène.
Comme pour `2026-07-fixture-attendue-derivee-avant-le-parseur`, la sortie attendue est dérivée par une implémentation de référence jetable (force brute, non versionnée) avec une assertion par cas qui épingle le phénomène visé ; chaque cours embarqué fait un aller-retour serde exact par `core::Course`.
Le solveur devra reproduire ces verdicts, jamais l'inverse.

Deux conséquences assumées de la sémantique swap : dans un horaire invalide, une alternative peut être valide sans rendre l'horaire entier valide (elle ne règle que ce cours) ; et une alternative qui exigerait de déplacer un second cours est marquée invalide même si un horaire global l'utilisant existe.

## Alternatives rejetées

- **Références par NRC seuls dans la sortie** : plus léger, mais l'UI devrait recouper chaque NRC dans le snapshot pour afficher les plages — une jointure côté vue qui frôle la logique métier.
- **Sémantique globale des alternatives** (valide ssi un horaire complet l'utilisant existe) : répond à une autre question que celle que l'UI pose au moment du changement de section, et son coût est celui d'une énumération par alternative.
- **Sélection « moins de conflits » (Max-CSP) dans le cas infaisable** : plus juste visuellement, mais la forme exacte est une question encore ouverte du plan ; « premier faisable, épinglés gardés » est déterministe et suffisant pour la v0.
- **Référencer les fixtures `courses/*.json` existants** : zéro duplication, mais les vraies plages ne produisent pas tous les phénomènes (dos-à-dos exact, conflit à trois cours aux paires compatibles) et un re-gel des pages casserait les cas.
