# US-80 — B-GMC, « Passage intégré au deuxième cycle »

**Persona** : Marilou, en génie mécanique, qui vise la maîtrise et veut créditer des cours de deuxième cycle.
**Intention** : voir ce que ce profil ajoute — et constater qu'il ne peut rien lui dire.

## Préconditions

- Programme « B-GMC », session d'admission « A26 ».

## Ce que le profil ajoute

- Aucun cours obligatoire.
- Règle 1 : sans contrainte, avec `courses: "negotiated"` — le répertoire ne publie aucune liste, les cours sont convenus avec la direction de programme.
- Une note en prose : « Conditions requises : avoir acquis 60 crédits du programme et présenter la moyenne de programme exigée selon l'entente. Rencontrer la direction de programme pour établir le choix de cours selon l'entente de passage intégré au deuxième cycle. »
- Aucun `credits_required`.

## Scénario

1. Marilou choisit « Passage intégré au deuxième cycle ».
2. Elle lit le panneau.

## Résultats attendus

- Le panneau affiche une carte « Règle 1 » contenant `Aucun cours défini pour cette règle.`
- La règle est bornée à 0 crédit — une règle sans contrainte exige la somme de ce qu'elle liste, soit rien : elle est donc satisfaite d'office et n'apparaît jamais en avertissement.
- Le bilan affiche une section `Passage intégré au deuxième cycle` sans total déclaré, ce qui est correct ici.

## Repères pour le test e2e

- La carte contient le texte `Aucun cours défini pour cette règle.`
- `#log-content` contient `Règle 1 : 0 cr. / 0 cr.` avec la classe `log-info`, jamais `log-warning`.

## Variantes et cas limites

- **Écart connu** : ni la note en prose ni le texte brut de la règle (`raw`) ne sont affichés. Marilou voit une carte vide et n'apprend nulle part qu'il lui faut 60 crédits et une rencontre avec la direction. L'invariant du projet — ne jamais rien perdre en silence — n'est pas respecté ici.
- Comparer avec le « Profil distinction » du B-GPH (US-88), qui est le même cas *avec* une contrainte de 12 crédits, et devient alors impossible à combler.
- Le mot-clé `negotiated` est une valeur reconnue du format, pas une anomalie : le frontend doit l'afficher comme telle plutôt que de la traiter en liste vide.
