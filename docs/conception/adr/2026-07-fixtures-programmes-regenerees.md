# Fixtures programmes régénérées depuis le HTML gelé

Date : 2026-07-20

## Contexte

Les trois fixtures programmes attendues ont été écrites à la main le 2026-07-13 (`aa0d051`) ; le HTML correspondant n'a été gelé que le 2026-07-17 (`b7319a0`).
Entre les deux, la page a bougé, et personne ne s'en est aperçu : aucun parseur ne les lisait encore.

Deux écarts, découverts en écrivant le parseur :

1. **`baccalaureat-en-genie-des-eaux`, Règle 4** — la fixture liste 14 cours, le HTML gelé en contient **19**.
   Le `ul.fe--liste-cours` de tête (`DDU-2000 ENT-1000 ENT-4020 GEX-3501 GGL-2601`), le seul qui ne soit précédé d'aucune étiquette thématique, manquait.
   `ENT-4020` et `GEX-3501` n'apparaissent dans aucun autre sous-groupe : ils étaient purement et simplement perdus.
   Aucune règle de lecture cohérente ne produit 14 — les Règles 1, 2 et 3 tirent justement tous leurs cours d'un `ul` sans étiquette.
2. **`maitrise-en-genie-des-eaux-avec-memoire`** — la règle « Recherche » portait `{min: 30, max: 30}`, alors que le bloc « Recherche » n'a **aucun** `span.fe-bloc-titre--credits`.
   La valeur avait été calculée (45 − 15). Une fixture qu'aucun parseur ne peut reproduire n'est pas un cas de test.

## Décision

1. Les six fixtures attendues (trois existantes, trois nouvelles : génie physique, industriel, mécanique) sont **produites par le parseur** à partir du HTML gelé, relues à la main contre la page, puis figées.
   Elles ne sont plus jamais écrites à la main : c'est l'écriture manuelle qui a produit la dérive.
2. Une page et sa fixture sont gelées **dans le même changement**. Un `.html` sans `.json` régénéré, ou l'inverse, est un défaut.
3. Écarts assumés sur les trois fixtures existantes, tous expliqués par les ADR de ce lot :
   - `notes` apparaissent (stages GCI-2580 / GEX-1580, exigences d'anglais, étiquettes de sous-groupes) ;
   - `raw` porte le paragraphe complet (la phrase DDU-1000 revient) ;
   - `mandatory: []` apparaît sur les concentrations ;
   - les apostrophes sont celles du source (U+2019), là où la saisie manuelle avait mis des `'` ASCII ;
   - `mandatory` de la maîtrise passe de 2 à 6, et sa règle « Recherche » disparaît.
4. Le test d'intégration porte, par page, la **liste des anomalies attendues** : une anomalie non listée fait échouer le test. Un parseur qui se met à renoncer sur une page qu'il lisait le dit.

## Alternatives rejetées

- **Corriger les fixtures à la main pour les faire passer** : reproduit exactement la cause de la dérive, et laisse le lecteur sans moyen de savoir laquelle des deux sources fait foi.
- **Re-télécharger les pages pour les faire correspondre aux fixtures** : la page vivante ne revient pas en arrière, et la fixture de la maîtrise n'aurait de toute façon jamais correspondu — sa valeur était calculée, pas lue.
