# Le fichier manuel d'un programme suit son millésime

Date : 2026-08-17

## Contexte

`data/programmes/{code}.manuel.json` n'avait pas de millésime dans son nom et le portait à l'intérieur, en champ `admission` que le `label` répétait (« H27 - Sciences de la nature »).
L'ADR qui l'avait décidé (`2026-07-annee-de-programme-selon-la-date-de-scrape`, point 4 : « reste sans année, une seule version de référence à la fois ») a été rendue caduque par `2026-08-millesime-de-programme-en-semestre` et `2026-08-plusieurs-millesimes-de-programme-offerts`.

Le résultat se voyait dans les données : `B-GEX.manuel.json` ne contenait qu'un cheminement, millésime A26, alors que `B-GEX-A24.json` existe.
Un étudiant admis en A24 se serait vu servir la grille A26 sans que rien ne le signale.

Le moment était le bon : `core::program_manual` était exporté mais appelé par aucune crate.
Renommer ne cassait aucun consommateur.

## Décisions

- **`data/programmes/{code}-{semester}.manuel.json`** — le nom du snapshot scrapé plus le suffixe, comme `B-GEX-A26.manuel.json`.
  Principe général : *un fichier manuel suit la forme de son homologue scrapé*.
  C'est aussi ce qui justifie l'asymétrie avec `data/cours.manuel.json`, qui reste un fichier unique parce que `cours.json` en est un.
- **Le champ `admission` de `CheminementType` disparaît** — le nom du fichier le porte — et le `label` perd son préfixe de millésime, qui ne faisait que le répéter.
  Les libellés du B-GMC redeviennent distinctifs : « Sciences de la nature », « Sciences de la nature - Profil international », « Technique de génie mécanique - Scolarité préparatoire complétée ».
  Un `label` vide est légitime : le millésime ne tient qu'une variante, et il n'y a rien à distinguer.
- **Conversion** : 3 fichiers fourre-tout → 19 fichiers, un par millésime.
  B-GEX 1, B-GIN 8 (A23→H27), B-GMC 10 (A22→H27, dont A26 qui porte 5 variantes).
  Aucun cheminement créé ni perdu : les 26 entrées étaient déjà là, repliées.
- **Le garde-fou du scraper est inchangé** : `cli.rs` filtre sur le suffixe `.manuel.json`, insensible au préfixe ; le test d'intégration le vérifie désormais sur un nom millésimé.

## Ce que le renommage rend visible

Cinq snapshots scrapés n'ont pas de compagnon manuel : `B-ANT-A26`, `B-GCI-A26`, `B-GEX-A24`, `B-GPH-A26`, `M-GEX-A26`.
Auparavant l'absence était cachée derrière un fichier fourre-tout et l'étudiant recevait la grille d'un autre millésime.
Désormais elle est explicite, et l'application doit dégrader proprement — aucun cheminement type proposé, jamais celui d'un autre millésime.

## Alternative rejetée

- **Garder un fichier par programme, millésimes en clé à l'intérieur** (la forme que `cours.manuel.json` vient d'adopter pour ses `vintages`) : moins de fichiers, mais le compagnon manuel cesserait de se lire comme son homologue scrapé, et le couple `(code, millésime)` — l'identité par laquelle toute l'app cherche un programme — ne suffirait plus à nommer le fichier.
