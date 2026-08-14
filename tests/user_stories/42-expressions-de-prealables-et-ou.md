# US-42 — Expressions de préalables ET / OU et prose hors grammaire

**Persona** : Théo, dont plusieurs cours ont des préalables à choix multiples.
**Intention** : que l'application respecte la logique des préalables plutôt que d'exiger tous les sigles cités.

## Préconditions

- Cours d'exemple :
  - `MAT-1900` → `MAT-0130 ET MAT-0150 ET MAT-0260` (conjonction pure).
  - `MAT-1910` → `MAT-1900 OU MAT-1920*` (disjonction avec simultané).
  - `FLS-2093` → expression parenthésée mêlant sigles et résultats d'examens en prose.
  - `CHM-1903`, `GCI-1000` → aucun préalable.

## Scénario

1. Théo place chacun de ces cours dans une session avancée, sans aucun préalable placé.
2. Il ajoute les préalables un à un et observe quand l'alerte disparaît.

## Résultats attendus

- Un `ET` exige tous ses membres; un `OU` en exige un seul.
- Les parenthèses de l'expression source sont respectées.
- Un cours sans préalables n'est jamais signalé.
- La prose non interprétable (résultats d'examens) est neutralisée et ne bloque pas à elle seule un cours dont l'expression contient un `OU`.
- L'infobulle liste les sigles absents, joints par ` ET `, même quand l'expression est une disjonction : la liste est indicative, pas la formule exacte.

## Repères pour le test e2e

- `MAT-1900` reste signalé tant que les trois préalables ne sont pas tous placés à gauche.
- `MAT-1910` cesse d'être signalé dès que `MAT-1900` seul est placé.
- Aucune erreur de console n'est produite par une expression exotique.

## Variantes et cas limites

- L'expression est convertie en JavaScript puis évaluée : toute expression qui ne se compile pas est considérée **satisfaite**, ce qui privilégie le faux négatif au faux positif. C'est le choix voulu, mais il masque les régressions du convertisseur — un test doit couvrir une expression volontairement tordue.
- Un sigle mal formé (`IFT 10426*`, avec une espace) est neutralisé plutôt que d'être traité comme un sigle (US-52).
- Les mots `ET` et `OU` ne sont convertis qu'entourés de limites de mot : un titre de cours contenant « OU » ne doit pas être transformé.
- **À venir** : le dépôt `generateur_horaire` produit déjà un arbre de préalables structuré (`prerequisites.parsed`, clés `all`/`any`); l'évaluation par expression de texte devrait à terme lui céder la place.
