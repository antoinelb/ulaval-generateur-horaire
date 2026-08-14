# US-74 — B-GIN, concentration « Systèmes productiques et distributiques »

**Persona** : Cynthia, en génie industriel, orientée vers la production et l'automatisation.
**Intention** : combler sa concentration à partir des cours de génie mécanique.

## Préconditions

- Programme « B-GIN », session d'admission « A26 ».

## Ce que la concentration ajoute

- Un cours obligatoire : `GMC-2007`.
- Règle 1 : 6 à 9 crédits parmi `GMC-1300`, `GMC-3016`, `GMC-4100`, `GMC-4200`, `GMC-4202`.
- Règle 2 : 3 à 6 crédits parmi `GGR-2109`, `GIN-4003`, `GLO-2005`, `GSO-2102`, `GSO-2104`.
- Règle 3 : 0 à 3 crédits parmi 7 cours (`GIF-1003`, `GIN-4021`, `GSO-2105`, `IFT-1003`, `MQT-2101`…).
- `credits_required` vaut 15.

## Scénario

1. Cynthia choisit cette concentration.
2. Elle place `GMC-2007`, deux cours de la Règle 1 et un de la Règle 2.
3. Elle vérifie que les cours `GMC-` de son programme de génie industriel sont bien offerts aux sessions qu'elle vise.

## Résultats attendus

- La concentration mêle des cours de trois matières (`GMC`, `GSO`, `GGR`) : chacune reçoit une teinte distincte.
- Les bornes inégales des Règles 1 et 2 s'affichent en intervalle.
- L'en-tête de section atteint 15 crédits.

## Repères pour le test e2e

- Trois couleurs de fond distinctes parmi les pastilles placées.
- `#log-content` contient `Règle 1 : … cr. / (6 à 9 cr.)`.

## Variantes et cas limites

- Les cours `GMC-` appartiennent d'abord au B-GMC : leurs préalables peuvent citer des cours absents du programme de génie industriel, ce qui produit des alertes de préalables manquants légitimes mais surprenantes.
- C'est la concentration du B-GIN qui recoupe le plus le B-GMC : bon terrain pour tester un changement de programme avec cours conservés (US-10).
