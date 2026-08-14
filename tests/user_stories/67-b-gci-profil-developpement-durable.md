# US-67 — B-GCI, « Profil développement durable »

**Persona** : Rémi, en génie civil, qui veut faire reconnaître son orientation en développement durable.
**Intention** : voir les 12 crédits du profil et les combler.

## Préconditions

Mêmes préconditions de déploiement qu'en US-63.

## Ce que le profil ajoute

- Un cours obligatoire : `DDU-1000`.
- Règle 1 : 6 crédits parmi `GAE-3006`, `GCI-4201`, `GBO-2040`.
- Règle 2 : 3 crédits parmi `GCI-3101`, `GCI-4301`.
- `credits_required` vaut 12 : c'est le seul profil des programmes de génie à en déclarer un, avec le profil entrepreneurial du B-GIN.

## Scénario

1. Rémi choisit « Profil développement durable ».
2. Il place `DDU-1000`, deux cours de la Règle 1 et un de la Règle 2.
3. Il lit le bilan.

## Résultats attendus

- Le bilan affiche `Profil développement durable : 12 cr. / 12 cr.` une fois les quatre cours placés.
- Les trois cartes apparaissent : les cours obligatoires du profil, la Règle 1 et la Règle 2.
- Placer un troisième cours de la Règle 1 n'augmente pas sa contribution : elle plafonne à 6 crédits.
- Les règles à deux ou trois cours produisent des cartes très courtes : la mise en page doit rester correcte.

## Repères pour le test e2e

- `.rule-card` du profil comptent respectivement 1, 3 et 2 `.course-line`.
- `#log-content` contient `Profil développement durable : 12 cr. / 12 cr.`

## Variantes et cas limites

- Un profil déclarant `credits_required` est le seul cas où la contribution de la section est plafonnée par un nombre déclaré plutôt que par la somme des maximums de ses règles.
- Choisir ce profil masque la concentration : Rémi ne voit plus ses 15 crédits de concentration dans le panneau, alors qu'il doit les faire aussi.
