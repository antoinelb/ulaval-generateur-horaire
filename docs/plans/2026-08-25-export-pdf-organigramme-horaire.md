# Export PDF : organigramme et horaire

## Goal

Deux boutons dans la bande entre le ruban de sessions et la grille hebdomadaire : « Exporter l'organigramme » produit un document dans le style de `gex_organigramme.pdf`, « Exporter l'horaire » produit les grilles hebdomadaires de toutes les sessions du plan à raison de deux par page, les deux via une vue d'impression dédiée et `window.print()` (l'étudiant fait « Enregistrer en PDF »).

## Out of scope

- Génération d'octets PDF en Rust/wasm : le chemin est l'impression navigateur (EXP-4), aucune nouvelle dépendance.
- Le logo Université Laval et les bandes de couleur facultaires du document officiel.
- Les détails du document officiel impossibles à calculer automatiquement : l'annotation « Attention : Planifier ensemble », les cases chevauchantes « ou » (BIO au choix), tout placement qui n'existe que dans la tête de la direction.
- Les préférences (jalon 10) et toute autre partie de l'UI.

## Constraints

- L'organigramme exporté est un rendu neuf, pas une capture du ruban existant : colonnes par session (A1→H8), cases de cours, jetons de préalables complets (lettre par cours-source, jeton de sortie à droite de la case source, jeton d'entrée à gauche des cases qui l'exigent, jeton ombré pour la concomitance) calculés depuis les arbres de préalables déjà parsés.
- Contenu : le placement réel de l'étudiant, enrichi des éléments du document officiel — boîtes « cours option » par règle avec les choix possibles, microprogrammes de stage, notes et légende des jetons, tableau des règles par cheminement ; les boîtes de cours option se placent dans l'espace restant d'une colonne une fois les cours obligatoires disposés, jamais par position hand-encodée.
- Quand le plan B-GEX est réinitialisé (aucune modification étudiante), le rendu doit correspondre au document générique officiel, aux éléments hors portée près.
- Pour un programme sans `cheminement_type` (importé, B-GMC…), le bouton exporte le placement affiché avec le même rendu ; la correspondance au document officiel n'est garantie que pour le B-GEX.
- Habillage : titre « Organigramme des cours », nom du programme, version/millésime, plus un message indiquant que le document est généré par l'application avec le lien vers le dépôt GitHub.
- PDF horaire : toutes les sessions du plan dans l'ordre, deux demi-pages par page ; une session sans horaire résolu garde sa demi-page avec titre, liste des cours placés et mention « horaire non publié », jamais omise.
- EXP-1 : les deux exports embarquent leur provenance — date de génération avec fuseau, `scraped_at` du snapshot, `BUILD_HASH`/`DATA_HASH`, lien GitHub.
- Toute la logique de modèle (jetons, disposition, pagination) est pure et testée nativement (dans `core` ou `present.rs`), rien dans les composants de vue ; `make test` reste à 100 %.
- Les boutons acquittent en moins de 100 ms (LAT) ; la vue d'impression n'altère jamais la mise en page à l'écran (LAY-2) ; états portés par glyphes et mots, jamais par la couleur seule (INP-3).
- Orientation par `@page` : paysage pour l'organigramme, l'horaire selon ce que la grille exige ; `@media print` masque l'app et ne montre que le document demandé.
- Domaine en français dans le texte affiché, identifiants en anglais dans le code.

## Items

1. La bande sous le ruban (`crates/ui/src/components/grid.rs` vers la ligne 91) reçoit les deux boutons d'export, au-dessus de la mention « cours hors grille ».
2. Un mécanisme d'impression dans `crates/ui/src/browser.rs` et le CSS : une vue d'impression montée cachée, une classe sur `body` choisissant quel document `@media print` révèle, `window.print()` déclenché par les boutons.
3. Un modèle pur « document organigramme » (dans `present.rs` ou `core`) : colonnes par session depuis le placement courant, cases de cours, attribution des lettres de jetons depuis les arbres de préalables, jetons d'entrée/sortie et concomitance, testé nativement.
4. Le placement des boîtes « cours option », microprogrammes de stage et de la boîte cours de langue dans l'espace restant des colonnes après les cours obligatoires, dans le même modèle pur.
5. Le composant de rendu organigramme : HTML/CSS reproduisant le style du document officiel (cases, jetons, en-tête, notes, légende, tableau des règles) sans logo, avec le message de provenance et le lien GitHub.
6. Un modèle pur « document horaire » : toutes les sessions du plan dans l'ordre, données de grille hebdomadaire par session, repli liste de cours + « horaire non publié » pour les sessions sans horaire, pagination deux par page, testé nativement.
7. Le composant de rendu horaire : deux demi-pages par page avec sauts de page CSS, grilles hebdomadaires imprimables, pied de provenance.
8. La provenance partagée des deux exports : date de génération avec fuseau, `scraped_at`, `BUILD_HASH`/`DATA_HASH`, lien vers le dépôt GitHub.
9. Un ADR documentant la décision : impression navigateur plutôt que génération PDF, rendu neuf calqué sur le document officiel, placement automatique des cours option, portée exclue.

## Acceptance

- Sur le B-GEX réinitialisé, l'aperçu d'impression de l'organigramme se compare côte à côte à `gex_organigramme.pdf` : mêmes colonnes, mêmes cases, mêmes jetons, boîtes cours option et habillage présents, sans les éléments hors portée.
- Le PDF horaire montre chaque session du plan, deux par page, avec les sessions sans horaire en liste + mention.
- Les deux exports portent leur provenance complète (EXP-1).
- `make lint && make test` passe avec 100 % de couverture.

## Check

`make lint && make test`
