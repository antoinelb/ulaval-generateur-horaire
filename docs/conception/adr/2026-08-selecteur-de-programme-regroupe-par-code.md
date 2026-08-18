# Le sélecteur de programme regroupe par code, le millésime est un select

Date : 2026-08-17

## Contexte

L'ADR `2026-08-plusieurs-millesimes-de-programme-offerts` avait tranché : « Le sélecteur de programme liste chaque `(code, millésime)` ».
À deux entrées B-GEX, la liste plate se lisait encore.

`2026-08-manifeste-de-programmes-genere` a fait passer le sélecteur de 8 à 24 entrées — conséquence qu'il assumait explicitement.
À cette taille la liste ne se lit plus : dix rangées portent littéralement « Baccalauréat en génie mécanique », huit « Baccalauréat en génie informatique ».
Le titre, seule chose que l'étudiante cherche, cesse de distinguer quoi que ce soit, et le millésime — un détail d'admission — devient le discriminant visuel.

## Décision

**Une rangée par code de programme**, avec un `select` des millésimes offerts à côté et un bouton « Choisir » distinct.

- Le regroupement est `panel::program_vintages`, pur et testé : codes dans l'ordre du snapshot, millésimes du plus récent au plus ancien.
- **L'ordre des millésimes suit le rang réel, jamais l'orthographe.** `parse_data` trie sur la chaîne « A26 », ce qui range tous les automnes avant tous les hivers : `H27` est plus récent que `A26` tout en le suivant dans le snapshot. `state::semester_rank`, extrait de `semester_precedes`, est la seule source de vérité de cet ordre.
- Titre et crédits de la rangée viennent du **millésime le plus récent**, celui que le select présélectionne : deux millésimes peuvent diverger, et la rangée doit annoncer celui qu'un clic immédiat donnerait.
- Le libellé reste le sigle brut (« A26 », « H27 »), comme partout ailleurs dans l'interface.
- Le `select` est un **frère** du bouton, jamais imbriqué : `<select>` dans `<button>` est du HTML invalide, et changer de millésime déclencherait le choix.
- Régler le select n'écrit **pas** dans le plan : un réglage sans effet n'est rien à annuler (ACT-1). Il vit dans un signal local jusqu'au clic, qui reste l'`edit_plan` annulable existant.

L'invariant d'appariement survit intact : `ProgramChoice` porte toujours `(code, semester)`, `chosen_program` cherche toujours sur le couple, et la trame de partage `ShareV1` n'est pas touchée.

Ce document supersède le 2ᵉ point de `2026-08-plusieurs-millesimes-de-programme-offerts` et la conséquence assumée de `2026-08-manifeste-de-programmes-genere`.

## Alternatives rejetées

- **La rangée entière cliquable, le select posé dessus** : moins de contrôles, mais la cible de clic autour du select devient ambiguë — cliquer « à côté » du menu choisirait le programme au lieu d'ouvrir la liste.
- **Deux menus déroulants en cascade** (programme, puis millésime), la forme du dépôt frère `grille-de-cheminement-interactive` : elle cache les programmes derrière un menu fermé, alors que le panneau entier est disponible pour les montrer tous depuis que le sélecteur le remplace.
- **Épeler le millésime** (« Automne 2026 ») : plus lisible isolément, mais rompt avec le vocabulaire de l'en-tête, du ruban et du réglage « Début », qui disent tous « A26 ».
- **Ne garder que le millésime le plus récent** : un étudiant fait son horaire sous la version de son admission — c'est le point de départ de `2026-08-plusieurs-millesimes-de-programme-offerts`.
