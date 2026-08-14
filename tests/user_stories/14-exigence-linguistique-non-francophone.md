# US-14 — Exigence linguistique, personne non francophone

**Persona** : Ana, étudiante internationale hispanophone admise au B-GEX.
**Intention** : intégrer `FLS-2093` à son cheminement et comprendre ce qui le précède.

## Préconditions

- Programme « B-GEX » : `language_requirement.non_francophone` pointe vers `FLS-2093` avec des seuils au TCF-TP.

## Scénario

1. Ana place `FLS-2093` en A26.
2. Elle lit l'infobulle et le journal.

## Résultats attendus

- `FLS-2093` est offert aux trois saisons : aucune alerte d'offre, quelle que soit la colonne.
- Ses préalables sont une longue expression parenthésée mêlant sigles (`FLS-2092`, `FLS-2062`) et résultats d'examens en prose.
- L'expression contient un `OU` de haut niveau : l'évaluation ne doit pas signaler le cours du seul fait que la prose n'est pas interprétable.

## Repères pour le test e2e

- `.dropped-tile[data-code="FLS-2093"]` n'a ni `cours-non-offert` ni `prerequis-manquants` lorsqu'aucun préalable n'est placé.
- Aucune entrée de `#log-content .log-error` ne mentionne `FLS-2093`.

## Variantes et cas limites

- Si l'évaluation devenait stricte, ce cours serait signalé à tort pour tous les étudiants internationaux : c'est le cas de non-régression à garder.
- Ana n'a pas de moyen de déclarer son score TCF-TP (même manque qu'en US-13).
- Le même mécanisme couvre les préalables qui citent des examens TOEFL, IELTS ou Versant.
