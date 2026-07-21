# Sections en groupes de choix

Date : 2026-07-19

**Points 2 et 5 remplacés le 2026-07-20 par `2026-07-sections-en-combinaisons-valides`** : l'hypothèse falsifiable du §5 a été falsifiée par 22 cours du catalogue complet, et le modèle plat du §2 devient `options: Vec<Vec<Section>>`, une liste d'inscriptions complètes. Les points 1, 3, 4 et 6 restent en vigueur — le §6 en particulier, dont la réconciliation arithmétique a débusqué le bogue de `2026-07-sections-de-premier-niveau-par-ascendance`. L'affirmation du contexte selon laquelle un NRC ne peut pas se répéter d'un groupe à l'autre est également fausse (CSO-6702, PSE-3501).

## Contexte

Le modèle initial découpait l'offre d'une saison en `Component { type: lecture | laboratory, sections }`, le `type` étant lu dans l'étiquette « Type: » de la page.
La lecture des fixtures gelées invalide cette hypothèse sur trois points.

« Type: » qualifie une **plage horaire**, pas une section : GAE-3008 NRC 84578 porte « Type: En classe » sur sa plage du mardi et « Type: Laboratoire » sur celle du mercredi — une seule inscription, deux séances hebdomadaires.
Le JSON attendu en tirait deux `Component` partageant le NRC 84578, ce qui se lisait « choisir 84578, puis choisir 84578 » : un NRC dupliqué dans deux groupes de choix est structurellement impossible, le NRC *étant* l'identité de l'inscription.

Le site encode « laboratoire » de deux façons : GCI-1007 en fait des NRC distincts à choisir (« Sections liées »), GCI-2010 en fait une plage supplémentaire du même NRC.
Aucune étiquette ne distingue ces deux cas de façon fiable — GCI-1007 annonce d'ailleurs « Type: En classe » sur ses laboratoires 2024 et 2025, et « Laboratoire » sur ceux de 2026.

Enfin GCI-2010 offre deux sections de premier niveau (A et B), ce qui écarte l'hypothèse « une section par cours et par saison ».

## Décision

1. `Component` et `ComponentKind` sont supprimés : la distinction cours/laboratoire n'est pas représentée, faute de source fiable, et elle ne porte aucune contrainte d'horaire.
2. `SeasonOffering { groups: Vec<Vec<Section>> }` : chaque groupe est un choix, on retient exactement une `Section` par groupe et on prend l'union de leurs plages.
   Le produit cartésien des groupes énumère donc les inscriptions valides — la contrainte vit dans la forme de la donnée, comme pour l'arbre des préalables.
3. Une `Section` garde `slots: Vec<Slot>` : GAE-3008 prouve qu'une section se donne deux fois par semaine, et `[]` couvre naturellement les sections à distance (ECN-4901).
4. Seules les plages **récurrentes** deviennent des `Slot` : « Dates: Du 19 jan. 2026 au 24 avr. 2026 » est retenue, « Date: 16 jan. 2026 » (singulier) est écartée.
   GCI-2010 section A juxtapose une « Rencontre » ponctuelle le vendredi 8 h 30 et son laboratoire hebdomadaire le vendredi 9 h : traiter la ponctuelle comme hebdomadaire créerait un conflit fantôme tout le trimestre.
   L'écart est délibéré et non silencieux — la plage est reconnue puis exclue, ce n'est pas une entrée hors grammaire.
5. L'hypothèse « les sections liées ne dépendent pas de la section de premier niveau choisie » est **assumée et vérifiable** : le modèle plat ne peut pas représenter un cours où la section A porterait les laboratoires 1-2 et la section B les laboratoires 3-4.
   Le parseur émet donc une anomalie si une saison présente plus d'une section de premier niveau **et** des sections liées.
   Le passage sur le catalogue complet (~10 000 cours) tranchera empiriquement ; aucune fixture connue n'exhibe ce cas.
6. Contre-vérification arithmétique : « N sections offertes » compte les sections de premier niveau seulement (1 pour GCI-1007 et ses 3 NRC, 2 pour GCI-2010).
   Un écart entre ce compte et le nombre de sections extraites est une anomalie — même patron de réconciliation que `2026-07-pagination-du-catalogue-par-comptage`.

## Conséquences

Les six fixtures `tests/fixtures/test_cases/courses/*.json` sont régénérées, GCI-2010 est ajouté comme cas de test (deux sections parallèles, plage ponctuelle à écarter, horaire « De 9h à 11h50 » sans minutes).

## Alternatives rejetées

- **Garder `ComponentKind`** : oblige à inventer une distinction que la source ne porte pas de façon fiable, et a déjà produit un NRC dupliqué dans le JSON attendu de GAE-3008.
- **Imbriquer les sections liées dans la section (`linked: Vec<LinkedSection>`)** : représente fidèlement le DOM et interdit par construction la combinaison invalide, mais force le solveur à traiter deux niveaux là où le produit cartésien suffit, pour un cas qu'aucune page connue n'exhibe.
- **Une liste plate de sections, sans groupes** : perd la contrainte « choisir exactement un laboratoire » de GCI-1007, le solveur pourrait en retenir deux ou aucun.
- **`Component | Vec<Component>` en enum non étiqueté** : un `Vec` à un élément exprime déjà « un seul », et un troisième `untagged` en chemin critique reconduit la dégradation silencieuse documentée dans `2026-07-prealables-hors-grammaire-en-enum`.
- **`slots: Option<Slot>`** : perdrait la seconde séance hebdomadaire de GAE-3008, et une séance perdue fait répondre « aucun conflit » sur un horaire qui en a un.
- **Conserver les plages ponctuelles avec un champ de récurrence** : rien n'est perdu, mais chaque site d'appel du solveur doit filtrer, et une omission redevient un conflit fantôme.
