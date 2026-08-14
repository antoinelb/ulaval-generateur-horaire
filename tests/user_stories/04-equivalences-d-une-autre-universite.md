# US-04 — Équivalences obtenues dans une autre université

**Persona** : Béatrice, admise au B-GEX après deux sessions en génie civil à l'Université de Sherbrooke.
**Intention** : marquer comme acquis les cours reconnus en équivalence, sans qu'ils occupent une session de sa grille.

## Préconditions

- Programme « B-GEX », session d'admission « A26 ».
- Le cheminement type A26 est chargé.

## Scénario

1. Béatrice glisse `MAT-1900`, `MAT-1910` et `GCI-1000` de leur session vers la colonne « Cours complétés ».
2. Elle vide les cases ainsi libérées en A26 et H27.
3. Elle observe le bilan des crédits et les préalables des cours de deuxième année.

## Résultats attendus

- Les trois pastilles sont dans la première colonne, qui n'est jamais vérifiée pour les préalables ni pour les conflits d'horaire.
- Leurs crédits comptent dans les crédits accumulés de **toutes** les sessions suivantes.
- Elles comptent dans les règles du programme au bilan des crédits, exactement comme si elles étaient placées dans une session.
- Les cours qui les exigent en préalable ne sont plus signalés.

## Repères pour le test e2e

- `.table-wrapper tbody tr td:first-child .dropped-tile[data-code="MAT-1900"]` existe.
- Aucun `.dropped-tile.prerequis-manquants` sur `MAT-1910` placé en session.
- La ligne `Cours obligatoires : … cr.` du journal inclut les crédits de ces trois cours.

## Variantes et cas limites

- Un cours reconnu en équivalence mais **absent du catalogue ULaval** doit passer par `cours-hors-catalogue.csv`; sans cela, la pastille n'a ni titre, ni crédits, ni couleur.
- La colonne « Cours complétés » n'a pas de code de session valide : aucun menu de section n'y apparaît, même pour un cours qui en a plusieurs.
- Un même sigle ne peut exister qu'une fois dans la grille : le déposer ailleurs le déplace, il ne se duplique pas.
