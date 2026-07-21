# Blocs de la page programme et groupe sans `<h3>`

Date : 2026-07-20

## Contexte

La section « Structure du programme » (`section#section-structure`) est faite de groupes (`div.fe-bloc-section`), chacun portant un `<h3>` facultatif et un ou plusieurs blocs (`div.collapsible-sections`, un par `h4.fe-bloc-titre--texte`).
Un bloc porte un titre, parfois « N crédits exigés », des accordéons (« Cours obligatoires » ou « Règle N – \<contrainte\> »), et parfois un paragraphe de prose.

Le `<h3>` nomme le rôle du groupe — « Concentrations », « Profils » — sauf sur le **bac en génie mécanique**, qui liste ses trois concentrations (« Cheminement sans concentration », « Robotique », « Génie du bâtiment durable ») dans un groupe **sans aucun `<h3>`**.
Or un groupe sans `<h3>` sert déjà à autre chose : « Autres exigences » (génie civil, physique, industriel) est un bloc de règles du programme.
Il faut donc distinguer les deux sans pouvoir se fier à l'étiquette.

## Décision

1. Le modèle de lecture est groupe → bloc → accordéon, dans l'ordre du document.
   `div.fe-bloc-section--paragraphe` est un **enfant direct** de `div.collapsible-sections` (vérifié sur les six pages gelées) : la prose appartient donc à un bloc précis, jamais au groupe.
2. Le rôle d'un groupe se décide ainsi :
   - `<h3>` = « Concentrations » → concentrations ; « Profils » → profils ;
   - pas de `<h3>`, premier groupe → blocs du programme ;
   - pas de `<h3>`, groupe suivant à **un seul bloc** → bloc du programme (« Autres exigences ») ;
   - pas de `<h3>`, groupe suivant à **plusieurs blocs** → concentrations, **et une anomalie est signalée** ;
   - `<h3>` inconnu → blocs du programme, et une anomalie est signalée.
3. Seul le **premier** bloc du programme nomme ses règles telles quelles (« Règle 1 »).
   Un bloc suivant préfixe les siennes de son titre (« Autres exigences – Règle 1 »), sinon deux « Règle 1 » se télescopent dans la même liste.
   Les concentrations et les profils gardent leurs titres nus : leurs règles vivent déjà dans un objet séparé.

L'hypothèse de cardinalité est signalée plutôt que silencieuse : une page qui la briserait le dirait dans `programmes_errors.log` au lieu de produire une structure fausse.

## Alternatives rejetées

- **Liste blanche par nom** (« Autres exigences » = règles, tout le reste = concentration) : dépend d'un libellé qu'ULaval peut changer sans préavis, et ne dit rien d'un troisième libellé.
- **Croiser avec « Concentrations offertes : »** de la section « En bref » : cette liste est **absente** du bac en génie civil et **incomplète** en génie mécanique (« Cheminement sans concentration » n'y figure pas). Elle ne peut donc pas servir de source.
- **Traiter tout groupe sans `<h3>` comme des blocs du programme** : produirait trois jeux de règles concurrentes en génie mécanique, et ferait mentir l'arithmétique des crédits (102 + 18 = 120, alors que la somme des trois concentrations donnerait 156).
