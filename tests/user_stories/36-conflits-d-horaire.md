# US-36 — Conflits d'horaire dans une session

**Persona** : Zachary, qui a mis cinq cours dans la même session sans regarder les plages horaires.
**Intention** : repérer et résoudre les chevauchements.

## Préconditions

- Une session contenant deux cours dont les plages se recoupent.

## Scénario

1. Zachary place les cours qui se chevauchent dans la même colonne.
2. Il lit le journal.
3. Il change la section de l'un des deux (US-34).

## Résultats attendus

- Les deux pastilles reçoivent la mise en évidence de conflit.
- Le journal indique le jour et la plage exacte du chevauchement, par exemple « … sont en conflit d'horaire dans la plage horaire du mardi de 10h30 à 11h20. »
- Le conflit est calculé pour chaque colonne de session indépendamment.
- La fenêtre de grille horaire montre la case en hachures bicolores avec les deux sigles.
- Changer de section fait disparaître le conflit si la nouvelle combinaison est libre.

## Repères pour le test e2e

- `.dropped-tile.cours-en-conflit` compte exactement deux éléments pour un conflit simple.
- `#log-content .log-error` contient une entrée mentionnant les deux sigles et le jour.
- Dans la fenêtre d'horaire, la case porte un `background` en `repeating-linear-gradient`.

## Variantes et cas limites

- Un cours en conflit avec lui-même — deux plages du même cours — n'est jamais signalé.
- Trois cours qui se chevauchent produisent trois paires de conflits; la fenêtre d'horaire n'affiche que les deux premiers sigles d'une case.
- Un cours sans horaire publié dans cette session est exclu du calcul, donc jamais en conflit.
- Le calcul est en O(n²) sur les cours de la colonne : avec huit cours, la vérification doit rester instantanée.
- Une plage dont l'heure est illisible est ignorée plutôt que de faire échouer la vérification.
