# Seul le ✕ ferme un message

Date : 2026-08-30

## Contexte

Depuis `2026-08-alertes-en-toasts-flottants`, « le clic n'importe où sur un message le ferme (note 12 conservée) ».
La carte entière portait donc un `onclick` de rejet, et chaque élément interactif à l'intérieur devait s'en défendre :

- le `<details>` « Détail technique » d'une erreur (ERR-3) appelait `stop_propagation` pour que déplier ne ferme pas le message qu'on cherchait justement à lire ;
- les deux boutons « ↶ Annuler » (`DocumentReset`, `LocalProgramRemoved`) faisaient de même, avec le commentaire expliquant que l'annulation devait lire sa charge utile avant que le rejet ne démonte la carte.

Trois esquives pour une seule cause. Et la cause était elle-même un piège : lire le détail d'une erreur, ou viser son « Annuler » et le manquer de deux pixels, effaçait le message — un rejet *mémorisé* par sujet, donc silencieux jusqu'à ce que son libellé change.

## Décision

Le rejet ne vit plus que dans le `✕`. La carte n'a plus d'`onclick`, et les trois `stop_propagation` disparaissent avec le parent qui les rendait nécessaires : un `<details>` redevient un `<details>`, un bouton redevient un bouton.

Deux conséquences suivies jusqu'au bout plutôt que laissées en dette :

- **INP-1.** Le `✕` passe de raccourci commode à seule sortie ; à 32 × 24 px il était sous la règle. Il monte à 48 × 48 px, comme le « ↶ Annuler » qui le jouxte (`.toast-undo`, `min-height: 3rem`). L'écart assumé de la bande de statut — la cible fait la hauteur de la rangée pour ne pas voler de hauteur à la grille — ne s'applique pas ici : un toast flotte, l'agrandir ne coûte rien à personne.
- **Clavier.** L'ancien rejet vivait sur un `div`, donc n'était atteignable qu'à la souris. Le `✕` est un `<button>` avec son `aria-label` : le seul chemin de rejet est désormais aussi un chemin clavier.

`cursor: pointer` quitte `.toast` : plus rien n'y est cliquable dans son ensemble, et le pointeur ne doit pas promettre le contraire.

Ce qui ne change pas : le rejet reste *par sujet et par libellé exact* (`2026-08-toasts-un-par-sujet-et-rejet-memorise`), les ✓ continuent de s'effacer seuls après 5 s, et les ⚠ et erreurs attendent toujours un rejet explicite (ALR-4).

## Alternatives rejetées

- **Garder le clic global et ajouter des esquives au cas par cas.** C'est l'état qu'on quitte : chaque nouvel élément interactif dans un message devait se souvenir d'appeler `stop_propagation`, et l'oubli ne se voyait pas au type-check — seulement à l'usage, en effaçant un message qu'on voulait lire.
- **Clic global, mais seulement sur les ✓.** Deux comportements de rejet selon la priorité du message, à distinguer à l'œil. LAY-3 : la même région se comporte de la même façon, la profondeur change, pas la mécanique.
- **Laisser le `✕` à 32 × 24 px.** Tenable tant que la carte entière était une cible de repli ; plus maintenant.

## Amende

- `2026-08-alertes-en-toasts-flottants` : « Le clic n'importe où sur un message le ferme (note 12 conservée) » est remplacé par le présent ADR. Le reste — pile flottante bas-droite, trois messages visibles puis « +N autres », auto-effacement des seuls ✓ — tient toujours.
