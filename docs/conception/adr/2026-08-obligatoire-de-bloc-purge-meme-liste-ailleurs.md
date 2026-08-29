# Un obligatoire de bloc part avec son bloc, même si le suivant le liste

## Contexte

L'ADR `2026-08-electifs-orphelins-purges-au-changement-de-bloc` définit l'orphelin par la **couverture** : un cours listé par le bloc quittant que *rien* sous la nouvelle portée ne liste.
Son amendement `2026-08-electifs-choisis-sous-le-bloc-partent-avec-lui` ajoute le filet des étiquettes : un électif **choisi sous** le bloc part avec lui, couvert ou non.

B-GMC A26 passe entre les deux (rapport persona 2026-08-29, reproduit deux fois) : « Cheminement sans concentration » → « Robotique » → retour, et GMC-3351 reste accroché à H8, le total figé à 114/120.
GMC-3351 est l'**unique obligatoire** de Robotique : le solveur le place tout seul dans `displayed_placement`, il n'est jamais un électif, donc il ne porte aucune étiquette — le filet de l'amendement ne le voit pas.
Et le bloc neutre le liste parmi la centaine de cours de sa Règle 1 : « couvert », donc gardé par la définition d'origine.
Le cours survit alors au changement et se réamorce par le seed du solveur (`request_json` verse les clés de `displayed_placement` dans les électifs de la requête).

## Décision

`panel::scope_orphans` sépare ce que le bloc quittant **imposait** de ce qu'il **offrait**, et les juge par deux tests distincts :

- un cours que le bloc quittant avait en `mandatory` ne survit que si la nouvelle portée l'**impose** aussi (obligatoire du programme, ou obligatoire d'un bloc choisi) ;
- un cours que le bloc quittant se contentait de **lister** survit dès que la nouvelle portée le liste (couverture inchangée).

Motif : un obligatoire de bloc n'a jamais été un choix de l'étudiante — c'est le bloc qui l'a mis là.
Figurer parmi cent options du bloc d'arrivée n'est pas l'avoir choisi ; garder le cours gonfle un total auquel on ne peut plus se fier, exactement le mensonge que l'ADR d'origine visait.

Le cas légitime ne bouge pas : un cours **listé** par les deux blocs reste (`GAE-1000`), et un cours injecté comme préalable et listé par une règle du programme (`GLO-1901`) n'appartient à aucun bloc, donc ne part avec aucun.

## Alternatives rejetées

- **Étiqueter aussi les cours auto-placés** (`elective_origins` posé par le solveur) : l'étiquette dit « choisi sous », et le solveur ne choisit pas ; il faudrait une deuxième provenance pour dire la même chose que `mandatory` dit déjà, dans le document persisté en plus.
- **Purger tout ce que le bloc quittant touchait**, listes comprises : renverse `2026-08-electifs-choisis-sous-le-bloc-partent-avec-lui` dans l'autre sens et reprend le cours qu'une étudiante avait bel et bien choisi dans une liste que les deux blocs partagent.
