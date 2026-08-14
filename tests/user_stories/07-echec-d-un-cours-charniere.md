# US-07 — Échec d'un cours charnière (MAT-1900)

**Persona** : Vincent, au B-GEX, qui a coulé `MAT-1900` Mathématiques pour l'ingénierie I à sa première session.
**Intention** : voir tout de suite l'effet en cascade de l'échec et replanifier.

`MAT-1900` est le préalable de `MAT-1910`, lui-même préalable de plusieurs cours de deuxième année.
Un seul échec décale une branche entière du cheminement.

## Préconditions

- Programme « B-GEX », session d'admission « A26 », cheminement type A26 chargé.
- `MAT-1900` est en A26, `MAT-1910` en H27.

## Scénario

1. Vincent déplace `MAT-1900` de A26 vers H27 : c'est sa reprise.
2. Il constate que `MAT-1910`, resté en H27, devient invalide.
3. Il déplace `MAT-1910` vers E27, puis vers A27 si le cours n'est pas offert l'été.
4. Il déplace ensuite les cours qui dépendaient de `MAT-1910`.

## Résultats attendus

- Dès l'étape 2, `MAT-1910` porte la bordure d'erreur et l'infobulle `Préalables manquants : MAT-1900`.
- Le journal ajoute une erreur nommant l'expression `MAT-1900 OU MAT-1920*`.
- Un préalable placé dans la **même** colonne ne suffit pas : `MAT-1900` en H27 laisse `MAT-1910` en H27 invalide.
- Placer `MAT-1920` — préalable simultané noté `*` — dans la même colonne que `MAT-1910` lève l'erreur, parce que l'expression est un `OU` (US-40).
- L'erreur disparaît dès que `MAT-1910` passe dans une colonne strictement à droite de `MAT-1900`.

## Repères pour le test e2e

- Après le déplacement, `.dropped-tile[data-code="MAT-1910"]` porte la classe `prerequis-manquants`.
- Son `title` vaut exactement `Préalables manquants : MAT-1900`.
- `#log-content .log-error` contient une entrée commençant par `MAT-1910 en H27 :`.

## Variantes et cas limites

- Un échec dans un cours d'appoint (`MAT-0150`) invalide toute la première année, pas seulement un cours.
- Deux échecs dans la même session doivent produire deux pastilles signalées et deux lignes de journal, sans que l'une masque l'autre.
- Vincent peut aussi garder `MAT-1900` dans « Cours complétés » s'il l'a repris et réussi hors grille : la vérification l'accepte alors pour toutes les sessions.
