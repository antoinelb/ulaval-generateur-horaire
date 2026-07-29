# Les équivalences se résolvent au millésime de session, fourni par l'appelant

Date : 2026-07-28

## Contexte

« Quand un cours a un équivalent, utiliser l'horaire du plus récent des deux » (`project_plan.md`).
La récence est un **millésime de session** : `data/cours/` garde un fichier par session (a2009…a2026), un cours dormant n'apparaît que dans le fichier de sa dernière offre, et son équivalent actif peut vivre dans un fichier plus récent.
Or `core::Course` ne porte aucune année — le millésime n'existe que dans le nom du fichier de données, que le cœur pur ne voit jamais.

## Décision

Une fonction pure `weekly::resolve_offering` reçoit, pour la saison visée, la paire `(offre, année)` du cours et celle de l'équivalent, chacune optionnelle, et rend la paire gagnante :

- le millésime le plus récent gagne ;
- à égalité, le cours lui-même gagne (jamais l'équivalent) ;
- un seul côté présent → ce côté ; aucun → rien.

Retourner la **paire** (et non l'offre seule) rend la fonction repliable quand un cours liste plusieurs équivalents : `equivalents.fold(paire_du_cours, …)`, l'égalité à gauche préservant la priorité du cours.
`build_domain` prend l'offre déjà résolue ; c'est l'appelant (UI, CLI), qui sait de quel fichier vient chaque offre, qui fournit les années.

## Alternatives rejetées

- **Porter l'année dans `Course`** : change le format du snapshot pour dupliquer une donnée déjà présente dans le nom du fichier, et la donnée n'a de sens qu'au chargement.
- **Toujours préférer le cours, repli sur l'équivalent seulement s'il n'a pas d'offre** : un cours dormant à l'horaire périmé masquerait l'horaire actif logé sous le code de l'équivalent — exactement le cas que la règle produit vise.
- **Résoudre dans `build_domain(course, season, …)`** : forcerait le passage d'un annuaire de cours et des millésimes dans la construction du domaine ; la résolution est une décision distincte, testable seule.
