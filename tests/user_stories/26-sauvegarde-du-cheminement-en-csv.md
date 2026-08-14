# US-26 — Sauvegarder le cheminement en CSV

**Persona** : Pierre-Luc, qui veut conserver sa grille avant d'essayer une autre organisation.
**Intention** : produire un fichier rechargeable.

## Préconditions

- Une grille contenant au moins une pastille.

## Scénario

1. Pierre-Luc clique « Sauvegarder un cheminement ».
2. Il clique « Sauvegarder le cheminement en format .csv ».

## Résultats attendus

- La fenêtre se ferme et un fichier `cheminement.csv` est téléchargé.
- Le fichier contient une ligne par colonne, dans l'ordre du tableau : d'abord `cours complétés`, puis chaque session.
- Chaque ligne commence par le libellé de la colonne, suivi des sigles de haut en bas, séparés par des points-virgules.
- Une colonne vide donne une ligne ne contenant que son libellé.
- Le fichier rechargé reproduit la grille à l'identique (US-24).

## Repères pour le test e2e

- Intercepter l'événement `download` de Playwright et vérifier `suggestedFilename() === 'cheminement.csv'`.
- Le contenu est encodé en UTF-8 et les lignes sont séparées par `\r\n`.
- Le nombre de lignes égale le nombre de `thead th`.

## Variantes et cas limites

- La sauvegarde ne conserve **pas** le programme, le millésime, la spécialisation, la section choisie par cours ni l'état de la case « Scolarité préparatoire ». Recharger dans un autre programme donne donc une grille valide mais un bilan différent.
- Une grille entièrement vide produit un fichier de libellés seuls, ce qui est un cas légitime.
- Aucun échappement n'est appliqué : un libellé contenant un point-virgule casserait le format, mais les codes de session n'en contiennent pas.
