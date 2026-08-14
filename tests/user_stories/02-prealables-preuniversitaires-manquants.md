# US-02 — Préalables préuniversitaires manquants (profil sciences de la santé)

**Persona** : Sarah, admise au B-GEX, sortant d'un DEC en sciences de la santé.
**Intention** : comprendre quels cours d'appoint elle doit faire avant les cours de première année.

Sarah a fait la biologie et la chimie, mais pas le calcul intégral ni l'algèbre linéaire.
Il lui manque donc `MAT-0150` et `MAT-0260` — et par ricochet tout ce qui en dépend.

## Préconditions

- Programme « B-GEX », session d'admission « A26 ».
- Le cheminement type A26 est chargé.

## Scénario

1. Sarah décoche « Scolarité préparatoire complétée ».
2. Elle déplie la section « Scolarité préparatoire » du panneau de droite.
3. Elle glisse `BIO-0150`, `CHM-0150`, `CHM-0160` et `CHM-0170` dans la colonne « Cours complétés ».
4. Elle laisse `MAT-0130`, `MAT-0150` et `MAT-0260` hors de la grille.
5. Elle lit les pastilles en erreur.

## Résultats attendus

- Toutes les pastilles dont les préalables dépendent des mathématiques d'appoint sont signalées : `MAT-1900`, `STT-1900`, `IFT-1903` et leurs descendants.
- L'infobulle d'une pastille signalée nomme les sigles manquants, par exemple `Préalables manquants : MAT-0130 ET MAT-0150 ET MAT-0260`.
- Le journal contient une ligne d'erreur par pastille signalée, citant l'expression logique complète du cours.
- Quand Sarah place ensuite `MAT-0150` en A26 et `MAT-0260` en H27, les erreurs disparaissent en cascade dans l'ordre des sessions, jamais avant.

## Repères pour le test e2e

- `#scolarite-completee` décoché déclenche une revérification complète.
- `.dropped-tile[data-code="MAT-1900"].prerequis-manquants` est présent.
- L'attribut `title` de cette pastille contient `Préalables manquants`.
- `#log-content .log-error` contient au moins une entrée mentionnant `MAT-1900`.

## Variantes et cas limites

- Un cours d'appoint placé **dans la même colonne** que le cours qui l'exige ne compte pas : seul un préalable simultané noté `*` le permet (US-40).
- Sarah peut aussi tout régler d'un coup en cochant la case, mais `MAT-0260` **n'est pas** dans la règle « Scolarité préparatoire » du B-GEX A26 : `MAT-1900` reste alors signalé. Le comportement attendu est à trancher — soit compléter la règle côté scraper, soit accepter l'alerte (voir US-38).
