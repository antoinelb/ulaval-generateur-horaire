# Conception du parseur de cours (préalables inclus)

Date : 2026-07-19

## Contexte

Étape « parseur de page cours » du plan : HTML → `core::Course`, y compris la grammaire des préalables (expressions OU/ET parenthésées, « Crédits exigés : N »).
Les règles de codage du projet interdisent la récursion et les boucles `while`, alors qu'une grammaire parenthésée appelle classiquement la descente récursive.
Les pages gelées portent plusieurs blocs de session pour une même saison (Automne 2024/2025/2026 sur GCI-1007) alors que `Course.seasons` est indexé par saison seule.

## Décision

1. La grammaire des préalables vit dans `parse/course.rs` (fonction pure, texte → `PrereqTree`, pas de HTML) ; le module `parse/prerequisites.rs` prévu est supprimé — même page, même artefact, un seul module (`docs/next_steps.md` mis à jour).
2. Analyse itérative à pile explicite : tokenisation, puis une boucle `for` bornée par le nombre de tokens ; une trame est empilée à chaque « ( » et repliée à la « ) » correspondante — profondeur et itérations bornées par la taille de l'entrée.
3. La fonction de grammaire retourne `Result<PrereqTree, ParseError>` ; l'assemblage du cours replie l'erreur en `Prerequisites::Raw` et la compte comme anomalie (l'anomalie est une donnée — patron du parseur de catalogue).
4. Par saison, seul le bloc de session le plus récent est retenu (hypothèse fondatrice « un snapshot par saison ») ; épinglé par la fixture GCI-1007, dont le JSON attendu porte les NRC d'Automne 2026.
5. L'état est une trame courante (`current`) hors pile plus une pile des trames englobantes (`enclosing`) : une trame courante existe toujours, et le `)` orphelin devient le `None` du `pop`, une vraie erreur d'entrée plutôt qu'un invariant à garder.
   Deux `expect` subsistent, aux deux repliages : les gardes `expecting_operand` ayant déjà rejeté un groupe sans opérande, `fold_frame` ne peut y retourner `None`.
   Exception assumée à la règle « éviter `expect` » : en `ok_or_else`, ces deux fermetures inatteignables restent à jamais non couvertes (llvm-cov les compte comme régions manquées, alors qu'un `expect` sur une ligne exécutée est couvert), et le message porte la preuve de l'invariant.

## Alternatives rejetées

- **Descente récursive à profondeur plafonnée** : plus proche du manuel, mais une exception aux règles de codage pour une grammaire à deux niveaux qui se replie bien en pile explicite.
- **Module `prerequisites.rs` séparé** : frontière artificielle pour une fonction pure consommée par un seul appelant.
- **Toutes les sessions dans `seasons`** : exigerait une clé saison + année, contraire au modèle de données et à l'hypothèse fondatrice.
- **Garder `ok_or_else` aux deux repliages (zéro `expect`)** : respecte la règle à la lettre, mais plafonne le fichier à ~98,7 % de couverture avec deux régions inatteignables à ré-expliquer à chaque exécution.
- **Rendre ces branches atteignables** (`or_groups: Vec<Vec<PrereqTree>>`, un OU pendant laissant un groupe vide détectable) : zéro `expect` et 100 % de couverture, mais un repliage plus complexe qui n'élimine qu'une des deux gardes.
