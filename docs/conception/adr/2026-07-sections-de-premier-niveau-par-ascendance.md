# Les sections de premier niveau se reconnaissent par ascendance, pas par filiation directe

Date : 2026-07-20

## Contexte

Le premier scrape complet a produit **une** ligne de cette forme :

```
Parsing …/drt-7104-sujets-speciaux-en-droit-des-affaires-i:
  Malformed entry for p.controls-title: Automne 2023 – 2 sections offertes: 1 top-level sections found
```

La page annonce deux sections et en contient bien deux, A (NRC 84328) et B (NRC 84329).
`data/cours/a2023.json` n'en porte qu'une : **le NRC 84329 a disparu du snapshot**, avec son horaire.

La cause est dans le HTML de la page, entre les deux sections :

```html
<div class="fe--message"><p><b>Droit de la concurrence<b></p></div>
```

Le second `<b>` est une coquille pour `</b>`.
Les deux `<b>` restent donc ouverts, et l'*adoption agency algorithm* de HTML5 les reconstruit autour de ce qui suit.
Vérification faite avec un parseur conforme :

```
section A (84328) → parents : [div.collapsible-sections, …]
section B (84329) → parents : [b, b, div.collapsible-sections, …]
```

`children()` ne retient que les enfants **directs** d'un élément.
Ce choix était délibéré et documenté : le commentaire de `parser/course.rs` explique qu'une section imbriquée ne vit que dans le wrapper `.dark`, et qu'un balayage restreint aux enfants directs ne peut donc pas déborder dans les sections liées.
Le raisonnement est juste sur du balisage bien formé, et faux dès qu'un élément de formatage non fermé décale la profondeur.

La perte n'a été détectée que par la réconciliation arithmétique de `2026-07-sections-en-groupes-de-choix` §6.
Aucun test, aucun type et aucune relecture ne pouvait la voir : la page se parse proprement, elle rend simplement une section de moins.

## Décision

Les sections de premier niveau d'une session sont les `div.toggle-section` **descendants** de la session dont aucun ancêtre, jusqu'à la session, n'est un `div.toggle-section--content-wrapper.dark`.

La propriété sur laquelle on s'appuie devient donc « ne pas être sous un wrapper `.dark` » — celle qui porte réellement le sens — au lieu de « être à une profondeur de 1 », qui n'en était qu'un proxy commode.
Le balayage remonte la chaîne d'ascendants, bornée par la profondeur du DOM; pas de récursion.

Le commentaire de module qui justifiait la filiation directe est corrigé pour dire *pourquoi* elle ne suffit pas.

## Conséquences

Mesuré avant écriture du code : DRT-7104 automne 2023 passe de 1 à 2 sections, IFT-1004 et PSE-3501 sont inchangés.

`drt-7104` rejoint les fixtures gelées.
Un test unitaire reproduit le cas isolément — et **le saut de ligne après la balise parasite y est essentiel** : le constructeur d'arbre HTML5 ne reconstruit les éléments de formatage ouverts qu'en insérant des données *caractères*, pas en insérant un `<div>`.
Le même balisage sans espace entre les éléments se parse proprement, et le test passerait alors contre le bogue même qu'il épingle.
Une note dans le test le dit, faute de quoi un « nettoyage » du HTML de test le désarmerait en silence.

La réconciliation arithmétique confirme sa valeur : c'est le seul garde-fou du parseur à avoir attrapé une perte de données réelle sur ~10 000 pages.
Ce genre de contrôle — écrire l'hypothèse comme une vérification à l'exécution plutôt que comme un commentaire — se généralise partout où la source annonce un compte.

## Alternatives rejetées

- **Nettoyer le HTML avant de le parser** (fermer les balises orphelines) : demanderait de réimplémenter une partie de la spécification HTML5 en amont d'un parseur qui l'implémente déjà correctement. Le DOM produit n'est pas faux, c'est notre lecture qui l'était.
- **Balayer tous les descendants et filtrer les sections liées par soustraction** (retirer de l'ensemble total celles qu'on a trouvées sous un `.dark`) : même résultat, mais deux balayages et un ensemble intermédiaire, et le critère reste implicite au lieu d'être écrit dans le filtre.
- **Signaler l'écart et s'arrêter là** : c'est l'état actuel, et il coûte une section par page touchée. L'anomalie a fait son travail en désignant le problème; la garder comme seule réponse reviendrait à documenter un bogue au lieu de le corriger.
- **Considérer le cas comme trop rare pour agir** (une page sur ~10 000) : la fréquence dépend d'une coquille dans le contenu rédactionnel du site, pas d'une propriété stable; la prochaine peut tomber sur un cours du GEX.
