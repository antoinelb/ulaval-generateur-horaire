# Préalables

Chaque cours porte ses préalables sous deux formes : le **texte source** (`raw`), toujours conservé, et — quand la grammaire le comprend — un **arbre ET/OU** (`tree`).

```json
"prerequisites": {
  "raw": "(GLG-1000 ET GLG-1900) OU GGL-2600",
  "tree": {"any": [{"all": ["GLG-1000", "GLG-1900"]}, "GGL-2600"]}
}
```

## Les nœuds de l'arbre

| Forme | Sens |
|---|---|
| `"GLG-1000"` | réussir ce cours (ou un équivalent) **avant** la session |
| `{"concomitant": "GCI-2010"}` | le `*` du répertoire : le cours peut être suivi **la même session** |
| `{"all": […]}` | ET : tous les enfants |
| `{"any": […]}` | OU : au moins un enfant |
| `{"program_credits": {"program": "GEX", "credits": 60}}` | avoir accumulé N crédits (du programme nommé, ou du sien si `program` est `null`) |
| `{"raw": "…"}` | un opérande hors grammaire, gardé verbatim |

La feuille chaîne et la feuille `concomitant` nomment le même cours : seule la précédence exigée diffère. Le répertoire marque la seconde d'une étoile (« GCI-2010* », glosée « préalable qui peut être suivi simultanément ») ; le texte `raw` la garde telle quelle, y compris sur un opérande hors grammaire, où elle reste dans le texte.

Un opérande `raw` — un test de classement, une plage de sigles (« ESG-2020 à 3799 ») — n'est jamais supprimé ni interprété : le placement le **présume satisfait** et le remonte dans `assumed`, pour que l'étudiant en juge lui-même.

`prerequisites` vaut `null` quand le cours n'en a pas ; la variante sans `tree` (`{"raw": "…"}` seul) signifie que la phrase entière est hors grammaire.

## Ce que le placement en fait

Dans un organigramme, un préalable doit être satisfait **avant** la session du cours — sauf une feuille `concomitant`, que la même session satisfait déjà.
Le réglage `concomitant: true` de la requête est une **dérogation** : il étend cette lecture à toutes les feuilles, étoilées ou non.
Un cours n'est jamais son propre préalable concomitant.
Un cours réussi (`passed`) satisfait ce qu'il préfigure ; un cours ni listé ni réussi rend l'opérande invérifiable, donc présumé et remonté.
Un arbre faux sous toute assignation (un code inexistant en `all`, un seuil de crédits inatteignable) rend le cours **implaçable** : il apparaît dans `blocked` avec `unsatisfiable-prerequisites`, preuve avant recherche.
