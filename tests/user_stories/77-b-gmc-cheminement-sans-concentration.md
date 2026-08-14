# US-77 — B-GMC, cheminement sans concentration

**Persona** : David, au baccalauréat en génie mécanique, qui veut garder toutes ses options ouvertes.
**Intention** : choisir ses 18 crédits à option dans la plus longue liste du programme.

## Préconditions

- Programme « B-GMC », session d'admission « A26 ».
- C'est la première concentration de la liste : elle est sélectionnée d'office.

## Ce que la spécialisation ajoute

- Aucun cours obligatoire.
- Règle 1 : 12 à 15 crédits parmi **51 cours** (`GMC-4054`, `GMC-4150`, `GMC-4151`, `GMC-4350`, `PHY-2100`…).
- `credits_required` vaut 18, soit plus que le maximum de sa seule règle.

## Scénario

1. David charge le B-GMC.
2. Il déroule la carte de la Règle 1.
3. Il place cinq cours et lit le bilan.

## Résultats attendus

- La carte affiche les 51 cours, chacun glissable; le panneau reste défilable et utilisable.
- La Règle 1 affiche `X cr. / (12 à 15 cr.)` et plafonne à 15.
- L'en-tête de section affiche `Cheminement sans concentration : X cr. / 18 cr.`

## Repères pour le test e2e

- La carte de la Règle 1 contient 51 `.course-line`.
- Après avoir placé six cours de trois crédits, la ligne du journal reste `Règle 1 : 15 cr. / (12 à 15 cr.)`.

## Variantes et cas limites

- **Incohérence de données à signaler** : la section déclare 18 crédits exigés mais sa seule règle en plafonne 15. La section ne peut jamais afficher `18 cr. / 18 cr.` — trois crédits viennent d'ailleurs, sans que le fichier le dise. À confronter à la page du répertoire.
- C'est la règle la plus longue de tous les programmes : bon cas de charge pour la recherche du panneau (US-33) et pour le temps de rendu.
- Le B-GMC A26 scrapé a perdu ses profils entrepreneurial et international et renomme une concentration : anomalie connue du parseur, documentée dans l'ADR `2026-08-conversion-des-millesimes-anciens`.
