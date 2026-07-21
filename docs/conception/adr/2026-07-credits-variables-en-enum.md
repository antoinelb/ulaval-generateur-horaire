# Les crédits variables sont un enum `Fixed | Range`

Date : 2026-07-20

## Contexte

Le premier scrape complet du catalogue a produit 265 enregistrements de la même forme dans `data/cours_errors.log` :

```
Parsing …/med-1911-stage-externat: Malformed entry for credits: 6 à 12
```

`parse_credits` lit la carte « Crédits » avec `raw.trim().parse::<u32>()`, et le `?` de `parse` jette la page entière quand la valeur n'est pas un entier.
265 cours — 259 MED, 4 PHA, 1 ESI, 1 DRT — sont donc absents de tout snapshot, horaire compris.

Ce n'est pas une dérive de balisage.
Ce sont des stages dont l'étudiant choisit le poids : la carte de MED-1911 « Stage-Externat » porte `<span class="promo-entete--titre">6 à 12</span>` sous le libellé « Crédits », et sa page est par ailleurs complète et cohérente.
Valeurs observées : `2 à 4` (251 fois), `3 à 4` (5), `0 à 6` (4), `6 à 12` (2), `0 à 4` (2), `3 à 9` (1).

Le séparateur est ` à ` en ASCII (U+00E0, pas d'espace insécable), le même mot que les bornes déjà lues par le tokenizer des préalables (`2026-07-bornes-de-credits-toutes-retirees`).

## Décision

`Course.credits: u32` devient `Credits`, un enum non étiqueté :

```rust
#[serde(untagged)]
pub enum Credits {
    Fixed(u32),
    Range { min: u32, max: u32 },
}
```

`Fixed(3)` sérialise `3`, `Range { min: 6, max: 12 }` sérialise `{"min":6,"max":12}`.
**Les quinze fixtures existantes gardent `"credits": 3` inchangé** : seule la forme objet est nouvelle, aucune donnée déjà produite ne change de forme.

`untagged` est sûr ici, contrairement au cas documenté dans `2026-07-prealables-hors-grammaire-en-enum` : les deux variantes sont un *nombre* et un *objet* JSON, des formes disjointes qu'aucune ne peut absorber.
La dégradation silencieuse y était acceptée faute de mieux; ici elle est structurellement impossible.

Les deux bornes sont conservées parce qu'aucune des deux n'est déductible de l'autre, et parce qu'un total affiché à l'étudiant pour un cours « 6 à 12 » est honnêtement un intervalle, pas un nombre.
Le choix de ce que la vue en fait — la borne basse, l'intervalle, une valeur saisie par l'étudiant — appartient à l'interface, pas au snapshot.

Les autres chemins d'échec de `parse_credits` restent des erreurs, parce qu'eux décrivent bien un balisage inattendu :

- carte présente mais sans `span.promo-entete--titre` → `MissingElement`
- valeur ni entière ni « N à M » (« trois ») → `MalformedEntry`
- **borne décroissante (« 4 à 2 ») → `MalformedEntry`**, validée explicitement : aucune page n'en porte, et l'accepter laisserait passer pour un fait du cours un intervalle qu'aucun étudiant ne peut satisfaire.

Aucune carte du tout vaut toujours `Fixed(0)` (`2026-07-cours-sans-carte-de-credits`).

## Conséquences

MED-1911 rejoint les fixtures gelées.
Il ne liste **aucune** session, ce qui en fait un cas de test isolé : l'intervalle est la seule chose que sa fixture affirme, rien d'autre ne peut masquer une régression dessus.

Le round-trip de `core` (`crates/core/tests/integration/course.rs`) l'ajoute à sa table, la forme objet n'étant exercée par aucune autre fixture.

## Alternatives rejetées

- **Garder `u32` et retenir la borne basse** : diff minimal, aucun consommateur touché, et le sens sûr pour un solveur (sous-compter refuse trop tôt plutôt que trop tard, comme `2026-07-bornes-de-credits-toutes-retirees`). Mais la borne haute est perdue sans trace, alors que la contrainte « ne jamais rien perdre en silence » vise exactement ce genre d'écrasement, et un total affiché à l'étudiant deviendrait faux pour 265 cours.
- **Un struct `Credits { min, max }` toujours sérialisé en objet** : uniforme, mais fait changer de forme les quinze fixtures et tous les snapshots déjà produits, pour représenter « min = max » dans l'écrasante majorité des cas.
- **Exclure les cours à crédits variables du catalogue**, comme les `0xxx` et `8xxx` : MED-1911 est un vrai cours inscriptible, pas un jalon administratif, et le sigle ne permet pas de décider — il faudrait visiter la page pour finir par la jeter.
- **Signaler une anomalie en plus de conserver le cours** : le journal se remplirait de 265 lignes pour un cas parfaitement compris et volontairement traité.
