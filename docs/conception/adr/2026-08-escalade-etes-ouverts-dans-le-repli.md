# L'escalade « étés ouverts » vit dans le repli wasm

## Contexte

Quand l'essai exact ne trouve rien (B-GMC : 8 sessions pleines, stage en sus qui n'entre qu'en été), le repli existant (`allow_unplaced`, ADR `2026-08-placement-au-mieux-en-repli`) remplissait au mieux en laissant des trous — alors qu'ouvrir les étés aurait tout placé.
Décision utilisateur du 2026-08-19 : réessayer étés ouverts avant de laisser des trous, l'été restant un dernier recours (ADR `2026-08-ete-en-dernier-recours-dans-lordre-des-valeurs`).

## Décision

`place_filling` devient `place_escalating`, trois passes dans `crates/wasm` (traversées par `generate` seulement — `verify` et `admissible` prouvent, jamais n'escaladent) :

1. exact, étés selon le réglage ;
2. si rien et étés fermés : exact, tous les étés ouverts ;
3. si toujours rien : relâchée (`allow_unplaced`, `max_solutions: 1`), étés ouverts.

Le rapport gagne `summers_forced` : les cours réguliers non épinglés assis dans un été que le réglage fermait, lus sur la solution gagnante (une passe qui ouvre sans utiliser ne déclare rien).
**La case « Ouvrir les étés » n'est jamais cochée par l'outil** : la décocher relancerait une résolution qui la recocherait — un combat que l'étudiant ne peut pas gagner. Le toast nomme les cours forcés et les leviers.

## Alternatives rejetées

- **Deuxième requête côté UI** : exigerait une deuxième empreinte de convergence, du bookkeeping `running`, et croiserait débounce et annulation — trois mécanismes fragiles pour le même résultat ; ici un seul endroit pur, testé nativement, et les consommateurs JS en profitent.
- **Cocher la case au succès de la passe 2** : voir ci-dessus — le réglage appartient à l'étudiant.
