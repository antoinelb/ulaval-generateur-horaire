# US-41 — Préalable exprimé en crédits accumulés

**Persona** : Sandrine, qui veut faire son premier stage dès sa deuxième session.
**Intention** : comprendre pourquoi l'application le refuse.

`GEX-1580` (stage) a pour seul préalable `Crédits exigés : 24`.
D'autres cours écrivent la même exigence sous la forme `1000 à 4999 Crédits exigés : 45`.

## Préconditions

- Programme « B-GEX », cheminement type chargé.

## Scénario

1. Sandrine déplace `GEX-1580` de E27 vers H27.
2. Elle lit l'alerte.
3. Elle le remet à E27.

## Résultats attendus

- L'alerte indique les crédits manquants : « Vous n'avez pas accumulé les 24 crédits requis en préalables à ce cours. Au début de la session H27, vous aurez accumulé N crédits. »
- L'infobulle de la pastille affiche `Crédits insuffisants : N/24 crédits accumulés`.
- Les crédits accumulés comptent tous les cours des colonnes **strictement à gauche**, colonne « Cours complétés » incluse.
- Les cours de la règle « Stages » sont **exclus** du décompte : leurs crédits sont en sus.
- La scolarité préparatoire cochée ajoute ses crédits au décompte.

## Repères pour le test e2e

- `.dropped-tile[data-code="GEX-1580"].prerequis-manquants` avec un `title` contenant `Crédits insuffisants`.
- Le message du journal cite le nombre exact de crédits accumulés.

## Variantes et cas limites

- Une exigence de crédits et des sigles manquants peuvent coexister : l'infobulle affiche alors les deux lignes.
- L'exigence est neutralisée dans l'expression logique et évaluée à part : un cours dont l'expression est `Crédits exigés : 24 OU MAT-1900` ne doit pas être faussement validé.
- La borne de cycle (`1000 à 4999`) n'est pas prise en compte : tous les crédits placés comptent, quel que soit le niveau des cours.
- Un cours placé en première colonne n'est jamais évalué, donc jamais bloqué par une exigence de crédits.
