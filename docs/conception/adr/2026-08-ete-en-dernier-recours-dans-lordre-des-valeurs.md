# L'été est un dernier recours dans l'ordre des valeurs

## Contexte

Avec les étés ouverts, l'ordre des valeurs de `value_ordered_domain` traitait un été comme n'importe quelle session : ascendant sans seed, par distance au seed sinon.
L'escalade du repli (ADR `2026-08-escalade-etes-ouverts-dans-le-repli`) ouvre les étés d'office quand l'essai exact échoue — sans démotion, elle y logerait des cours réguliers qui tiendraient très bien à l'automne ou à l'hiver.

## Décision

Pour un cours régulier non épinglé, l'été passe en fin d'ordre : clé de tri `(été, distance-au-seed, session)` avec seed, `(été, session)` sans (le tri sans seed devient explicite).
Un stage n'est jamais dému (l'été est son domicile) ; un pin est un singleton.
**La démotion prime le seed** : un cours semé en été par une escalade antérieure ressort de l'été dès qu'une place régulière se libère — pas d'oscillation, la recherche est déterministe et l'empreinte de convergence absorbe la réponse.
L'ensemble complet des solutions ne change pas — seul l'ordre d'énumération, donc la première solution (`max_solutions: 1`), bouge ; les fixtures gelées comparent des ensembles triés et ne sont pas régénérées.

## Alternatives rejetées

- **Exempter l'ancre du seed de la démotion** : un cours un jour placé en été y resterait collé même quand la cause (plafond, cours retiré) a disparu.
- **Pénalité pondérée (distance + K en été)** : un réglage arbitraire de plus, pour le même effet que la démotion lexicographique.
