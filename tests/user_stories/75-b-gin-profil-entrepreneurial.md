# US-75 — B-GIN, « Profil entrepreneurial »

**Persona** : Charles, en génie industriel, qui veut démarrer son entreprise pendant ses études sans renoncer à sa concentration.
**Intention** : voir les 12 crédits du profil et les combler, une concentration restant sélectionnée en même temps.

## Préconditions

- Programme « B-GIN », session d'admission « A26 ».

## Ce que le profil ajoute

- Trois cours obligatoires : `ENT-1000`, `ENT-3000`, `ENT-3010`.
- Règle 1 : 3 crédits parmi `ENT-2023` et `ENT-2145`.
- `credits_required` vaut 12.
- Aucun cours `ENT-` ne figure dans les règles d'une concentration du B-GIN : contrairement au « Profil développement durable » du B-GCI (US-67), ce profil ne recoupe aucune concentration ici.

## Scénario

1. Charles choisit la concentration « Approche généraliste » dans le premier menu, puis le profil « Profil entrepreneurial » dans le second — les deux menus du panneau sont indépendants et changeables en tout temps.
2. Il place les trois cours obligatoires et `ENT-2023`.
3. Il lit le bilan.

## Résultats attendus

- Le groupe « Profil — Profil entrepreneurial » affiche `12/12 cr` une fois les quatre cours placés.
- Deux cartes apparaissent sous ce groupe : « Cours obligatoires » et la Règle 1.
- Tous les cours du profil partagent la matière `ENT` et donc la même teinte.
- Le groupe « Concentration — Approche généraliste » reste affiché juste au-dessus, avec sa propre progression ; aucun de ses cours ne recoupe ceux du profil, donc rien n'y change quand Charles place les quatre cours du profil.

## Repères pour le test e2e

Les sélecteurs `.course-line` et `#cheminement-select` sont ceux du DOM de l'application JS soeur (`grille-de-cheminement-interactive`).
Les textes cités entre guillemets — « Profil — … », « Concentration — … », les progressions `X/Y cr` — sont ceux de l'UI Rust (`crates/ui/src/panel.rs`), qui offre deux menus `panel-knob` séparés (« Concentration », « Profil ») plutôt qu'un menu combiné.

- `#cheminement-select` contient « Profil entrepreneurial » puis « Profil international », après les cinq concentrations.
- La carte des cours obligatoires contient 3 `.course-line`, la Règle 1 en contient 2.
- Le badge du groupe « Profil — Profil entrepreneurial » affiche `12/12 cr`.

## Variantes et cas limites

- La note de la concentration « Approche généraliste » renvoie à ce profil, mais elle n'est affichée nulle part (US-70).
- Choisir le profil ne masque plus la concentration : les deux menus se choisissent et se changent indépendamment dans l'UI Rust ; ici, aucun cours n'est partagé entre le profil et une concentration, donc les deux progressions restent simplement côte à côte sans rien compter en double.
- `ENT-3020` figure dans le fichier hors catalogue du B-GEX à 0 crédit; ne pas le confondre avec `ENT-3000` et `ENT-3010` de ce profil.
