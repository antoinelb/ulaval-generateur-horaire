# US-75 — B-GIN, « Profil entrepreneurial »

**Persona** : Charles, en génie industriel, qui veut démarrer son entreprise pendant ses études.
**Intention** : voir les 12 crédits du profil et les combler.

## Préconditions

- Programme « B-GIN », session d'admission « A26 ».

## Ce que le profil ajoute

- Trois cours obligatoires : `ENT-1000`, `ENT-3000`, `ENT-3010`.
- Règle 1 : 3 crédits parmi `ENT-2023` et `ENT-2145`.
- `credits_required` vaut 12.

## Scénario

1. Charles choisit « Profil entrepreneurial ».
2. Il place les trois cours obligatoires et `ENT-2023`.
3. Il lit le bilan.

## Résultats attendus

- Le bilan affiche `Profil entrepreneurial : 12 cr. / 12 cr.` une fois les quatre cours placés.
- Deux cartes apparaissent : « Cours obligatoires (Profil entrepreneurial) » et la Règle 1.
- Tous les cours du profil partagent la matière `ENT` et donc la même teinte.

## Repères pour le test e2e

- `#cheminement-select` contient « Profil entrepreneurial » puis « Profil international », après les cinq concentrations.
- La carte des cours obligatoires contient 3 `.course-line`, la Règle 1 en contient 2.
- `#log-content` contient `Profil entrepreneurial : 12 cr. / 12 cr.`

## Variantes et cas limites

- La note de la concentration « Approche généraliste » renvoie à ce profil, mais elle n'est affichée nulle part (US-70).
- Choisir le profil masque la concentration : Charles ne voit plus les 15 crédits de concentration qu'il doit faire aussi. C'est la limite structurelle du menu à choix unique, commune à tous les programmes.
- `ENT-3020` figure dans le fichier hors catalogue du B-GEX à 0 crédit; ne pas le confondre avec `ENT-3000` et `ENT-3010` de ce profil.
