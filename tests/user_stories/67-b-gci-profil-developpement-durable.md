# US-67 — B-GCI, « Profil développement durable »

**Persona** : Rémi, en génie civil, qui veut faire reconnaître son orientation en développement durable sans renoncer à sa concentration.
**Intention** : voir les 12 crédits du profil et les combler, en gardant la concentration « Eau et environnement » sélectionnée en même temps.

## Préconditions

Mêmes préconditions de déploiement qu'en US-63.

## Ce que le profil ajoute

- Un cours obligatoire : `DDU-1000`.
- Règle 1 : 6 crédits parmi `GAE-3006`, `GCI-4201`, `GBO-2040`.
- Règle 2 : 3 crédits parmi `GCI-3101`, `GCI-4301`.
- `credits_required` vaut 12 : c'est le seul profil des programmes de génie à en déclarer un, avec le profil entrepreneurial du B-GIN.
- `GCI-4201`, `GCI-3101` et `GCI-4301` figurent aussi dans la Règle 1 de la concentration « Eau et environnement » (US-64) ; `GBO-2040` et `GAE-3006` figurent dans la Règle 1 du « Cheminement sans concentration ».

## Scénario

1. Rémi choisit la concentration « Eau et environnement » dans le premier menu, puis le profil « Développement durable » dans le second — les deux menus sont indépendants et changeables en tout temps.
2. Il place `DDU-1000`, `GCI-4201` et `GBO-2040` (Règle 1 du profil), puis `GCI-3101` (Règle 2 du profil).
3. Il lit le bilan.

## Résultats attendus

- Le groupe « Profil — Profil développement durable » affiche `12/12 cr` une fois les quatre cours placés.
- Trois cartes apparaissent sous ce groupe : les cours obligatoires du profil, la Règle 1 et la Règle 2.
- Placer un troisième cours de la Règle 1 du profil n'augmente pas sa contribution : elle plafonne à 6 crédits.
- Les règles à deux ou trois cours produisent des cartes très courtes : la mise en page doit rester correcte.
- `GCI-4201` et `GCI-3101` comptent aussi dans la Règle 1 de la concentration « Eau et environnement » : les portées concentration et profil sont indépendantes, un même cours crédite les deux à la fois.
  Ni l'un ni l'autre n'affiche « compté dans la Règle N » — ce sous-titre ne s'applique qu'entre règles d'une même portée, pas entre la concentration et le profil.
- Le groupe « Concentration — Eau et environnement » affiche sa propre progression (`6/15 cr` avec ces deux seuls cours placés côté concentration), qui n'est diminuée par rien de ce que le profil a réclamé.

## Repères pour le test e2e

Les sélecteurs `.course-line` et `.rule-card` sont ceux du DOM de l'application JS soeur (`grille-de-cheminement-interactive`).
Les textes cités entre guillemets — « Profil — … », « Concentration — … », les progressions `X/Y cr` — sont ceux de l'UI Rust (`crates/ui/src/panel.rs`) ; l'application JS peut ne pas encore les porter mot pour mot.

- `.rule-card` du profil comptent respectivement 1, 3 et 2 `.course-line`.
- Le badge du groupe « Profil — Profil développement durable » affiche `12/12 cr`.
- `.course-line` de `GCI-4201` apparaît à la fois dans la carte de la Règle 1 du profil et dans celle de la Règle 1 de la concentration, sans sous-texte « compté dans la Règle N » dans aucune des deux.

## Variantes et cas limites

- Un profil déclarant `credits_required` est le seul cas où la contribution du groupe est plafonnée par un nombre déclaré plutôt que par la somme des maximums de ses règles.
- Choisir ce profil ne masque plus la concentration : les deux menus se choisissent et se changent indépendamment, et le panneau affiche les deux groupes en même temps.
