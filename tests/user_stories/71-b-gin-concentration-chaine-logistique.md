# US-71 — B-GIN, concentration « Ingénierie de la chaîne logistique et des réseaux de création de valeur »

**Persona** : Jean-Philippe, en génie industriel, qui vise la logistique et le transport.
**Intention** : combler une concentration à quatre règles aux bornes inégales.

## Préconditions

- Programme « B-GIN », session d'admission « A26 ».

## Ce que la concentration ajoute

- Un cours obligatoire : `GIN-4021`.
- Règle 1 : 3 à 9 crédits parmi `GSO-2102`, `GSO-2104`, `GSO-2105`.
- Règle 2 : 3 à 9 crédits parmi 7 cours (`GGR-2109`, `GIN-4003`, `GLO-2005`, `GSO-2106`, `MNG-2100`…).
- Règle 3 : 0 à 6 crédits parmi 11 cours (`GIF-1003`, `GMC-1300`, `GMC-2007`…).
- Règle 4 : 0 à 3 crédits, un seul cours listé (`MNG-3103`).
- `credits_required` vaut 15.

## Scénario

1. Jean-Philippe choisit cette concentration.
2. Il place `GIN-4021`, un cours de la Règle 1 et deux de la Règle 2.
3. Il lit le bilan.

## Résultats attendus

- Les règles à bornes inégales sont affichées sous la forme `X cr. / (3 à 9 cr.)`.
- Une règle dont le minimum vaut 0 (Règles 3 et 4) n'est jamais en avertissement, même vide.
- Le titre long de la concentration s'affiche entièrement dans le menu, l'en-tête de section et l'en-tête de chaque carte de règle, sans casser la mise en page.
- La somme des maximums des règles dépasse largement les 15 crédits déclarés : la contribution de la section reste plafonnée à 15.

## Repères pour le test e2e

- `#log-content` contient une ligne au format `Règle 1 : … cr. / (3 à 9 cr.)`.
- L'option de `#cheminement-select` porte le titre complet de la concentration.
- La contribution de section ne dépasse jamais 15 dans le journal.

## Variantes et cas limites

- Une règle à un seul cours (`MNG-3103`) borné 0 à 3 est le cas dégénéré à vérifier : elle doit s'afficher et se comptabiliser normalement.
- `GIN-4021` apparaît à la fois en cours obligatoire ici et dans la Règle 1 de l'« Approche généraliste » : un même sigle change de rôle selon la concentration.
