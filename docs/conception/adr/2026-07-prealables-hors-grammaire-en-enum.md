# Préalables hors grammaire : enum `Parsed | Raw`

Date : 2026-07-19

## Contexte

Le parseur de cours doit conserver un préalable dont le texte sort de la grammaire : `{"raw": "…"}` sans arbre, affiché et signalé, jamais ignoré (CONCEPTION § Préalables).
`core::Prerequisites` est un struct qui exige `tree` ; « brut sans arbre » y est irreprésentable.

## Décision

`Prerequisites` devient un enum non étiqueté (`#[serde(untagged)]`) :
`Parsed { raw, tree }` sérialisé `{"raw": …, "tree": …}`, et `Raw { raw }` sérialisé `{"raw": …}`.
Même patron que `RuleCourses` (ADR `2026-07-reference-de-regle-structuree`) : le type n'admet que les combinaisons légales, et un arbre sans texte source est irreprésentable.
Limite connue de `untagged` : à la désérialisation, un `tree` malformé retombe silencieusement sur `Raw` (les variantes sont essayées dans l'ordre, et `Raw` ignore les champs inconnus) ; accepté parce que les snapshots sont produits par notre propre sérialiseur, et épinglé par un test.

## Alternatives rejetées

- **`tree: Option<PrereqTree>`** : diff minimal, mais la distinction « parsé / hors grammaire » repose sur une convention (`None`) plutôt que sur le type ; l'enum force chaque consommateur à traiter le cas hors grammaire et suit le précédent `RuleCourses`.
- **Garder le struct et signaler l'anomalie hors données** : le texte brut disparaît du snapshot, violant « conservé, affiché, jamais ignoré ».
