# Harnais UI de débogage dans un crate dédié (`crates/ui-debug`)

## Contexte

Les solveurs A et B et le vérificateur de couverture sont finis et testés, mais rien ne permettait de les exercer visuellement : `crates/ui` est un squelette vide et le CLI ne montre ni les conflits sur une grille, ni le temps de calcul, ni les sélecteurs concentration/profil.
Il fallait un outil de test rapide (deux pages : Horaire, Organigramme) avant d'entamer les jalons UI réels (3–9).

## Décision

Le harnais vit dans un nouveau crate `crates/ui-debug` (Dioxus 0.7, web seulement) et `crates/ui` reste intact pour la vraie UI.
Il est exclu de la couverture par la regex du makefile (`crates/ui-debug/`) : c'est de la vue jetable, toute logique mesurable vit dans `core` (invariant « aucune règle métier dans la vue », vérifié par l'extraction `core::intake`).
`place()` tourne dans le fil principal du navigateur : l'onglet gèle pendant la recherche — assumé pour un outil de test, borné par des budgets par défaut plus bas que le CLI (1 M nœuds, 1 000 solutions, modifiables).
Le chronométrage utilise `web-time` (l'`Instant` de `std` panique en WASM) et n'entoure que l'appel au solveur, jamais l'intake ni le rendu.

## Alternatives rejetées

- Construire le harnais dans `crates/ui` : aurait pollué l'historique et imposé une structure prématurée à la vraie UI.
- Web worker pour `place()` : garde l'onglet réactif mais exige un second binaire WASM et du message-passing — injustifié pour un harnais.
- Couvrir `ui-debug` par des tests : la vue ne contient que du rendu ; les fonctions testables ont été déplacées dans `core::intake` où elles sont couvertes à 100 %.
