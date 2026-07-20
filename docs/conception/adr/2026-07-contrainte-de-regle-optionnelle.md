# Contrainte de règle optionnelle

Date : 2026-07-20

## Contexte

`Rule.constraint` était un `Constraint` obligatoire, lu dans l'en-tête de l'accordéon : « Règle 1 – Un cours parmi : » → `{count: 1}`, « Règle 2 – 3 à 9 crédits parmi : » → `{min: 3, max: 9}`.

Le profil « Passage intégré au deuxième cycle » du bac en génie mécanique écrit un en-tête coupé en pleine phrase :

```html
<span class="item">Règle 1 – Réussir la scolarité de</span>
…
<p class="fe-bloc-regle--ligne"> deuxième cycle suivante : </p>
```

Aucun nombre nulle part, aucune carte de cours, et le bloc ne porte pas de « N crédits exigés ».
La page était donc **irreprésentable**.

Deux autres formes, elles, sont bien des contraintes mais ne suivaient pas la grammaire connue : « Règle 1 – 3 crédits » sans « parmi : » (génie physique, industriel, mécanique), et un minimum nul (« Règle 3 – 0 à 12 crédits parmi : »).

## Décision

1. `Rule.constraint` devient `Option<Constraint>`, avec `#[serde(default, skip_serializing_if = "Option::is_none")]` — même patron que `Profile.credits_required` : la clé disparaît quand la contrainte est absente.
   Une règle sans contrainte reste affichée à l'étudiant ; le solveur ignore ce qu'il ne peut pas compter.
2. Un en-tête dont la contrainte est illisible produit une anomalie nommant l'en-tête complet, jamais une valeur par défaut.
3. La grammaire des contraintes accepte les six formes réellement rencontrées, en tolérant la queue « parmi : » comme facultative :

   | motif | contrainte |
   |---|---|
   | `Un cours parmi :` | `{count: 1}` |
   | `X crédits parmi :` / `X crédits` | `{min: X, max: X}` |
   | `X à Y crédits parmi :` | `{min: X, max: Y}` |

## Alternatives rejetées

- **Une variante `Constraint::Raw { raw }`** : garde `constraint` obligatoire, mais duplique le texte source déjà conservé dans `RuleCourses::Raw`, et oblige tout consommateur à filtrer une variante qui ne contraint rien.
- **Écarter la règle et journaliser** : contredit l'invariant « ne jamais perdre une entrée non reconnue » — c'est la seule règle de ce profil, l'écarter viderait le profil.
- **Fabriquer `{count: 1}`** : c'est ce qu'avait proposé l'analyse automatique de la page ; la valeur n'a aucun fondement dans le source et un solveur la prendrait au sérieux.
