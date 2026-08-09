# Le millésime d'un programme est automne ou hiver, jamais été

Date : 2026-08-09

## Contexte

L'ADR `2026-08-millesime-de-programme-en-semestre` (point 2) datait un scrape de la session qui le suit en trois bandes : janvier–avril → `E`, mai–août → `A`, septembre–décembre → `H` de l'année suivante.
Or un programme n'est jamais défini pour l'été : les versions de programme sont publiées pour l'automne ou l'hiver seulement.
La bande `E` produisait donc des millésimes qu'aucun programme réel ne porte.

## Décision

`semester_after` passe à deux bandes : septembre–décembre → `H` de l'année civile suivante, tous les autres mois (janvier–août) → `A` de l'année courante.
Cette règle remplace le point 2 de `2026-08-millesime-de-programme-en-semestre` ; le reste de cet ADR (format `A26`, `--semester`, `possible_semester_start`) est inchangé.
Le plancher au jour zéro pour une horloge d'avant 1970 donne désormais `A70`.
Aucune migration : les snapshots existants sont tous `A26` (scrapés en juillet–août), même millésime sous l'ancienne et la nouvelle règle.
Les dates de scrape des programmes du calendrier (`data/dates_scraping.txt`, ADR `2026-08-scraping-pilote-par-fichier-de-dates`) tombent juste : 10 novembre → hiver suivant, 1er avril → automne courant.

## Alternatives rejetées

- **Interdire `E<yy>` au flag `--semester`** : la règle est une présomption de datation, pas une contrainte du domaine des re-runs épinglés — un re-run explicite peut viser ce qu'il veut ; `Season::Summer` reste par ailleurs pleinement valide côté sessions d'admission et offres de cours.
- **Garder trois bandes** : produirait des millésimes `E` qu'aucun programme ne porte, et un scrape de janvier–mars viserait un « été » au lieu de l'automne réellement préparé.
