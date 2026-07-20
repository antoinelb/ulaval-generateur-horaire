# Mode hybride : la moitié à distance n'occupe aucune plage

Date : 2026-07-19

## Contexte

L'ADR `2026-07-extraction-html-de-la-page-cours` § 2 ne reconnaissait que deux modes dans l'en-tête `button.header-wrapper` : « En classe » et « À distance ».
Le premier scrape réel a rencontré un troisième, « Hybride » — 24 anomalies sur GEX + GCI, dont GEX-3100.
Chaque section touchée était écartée en entier, donc son horaire perdu.

Une section hybride liste **deux plages** :

```
<li …><strong>Type:</strong> Sur Internet</li>
<li …><strong>Dates:</strong> Du 6 sept. 2022 au 16 déc. 2022</li>

<li …><strong>Type:</strong> En classe</li>
<li …><strong>Dates:</strong> Du 6 sept. 2022 au 16 déc. 2022</li>
<li …><strong>Journée:</strong> Mardi</li>
<li …><strong>Horaire:</strong> De 9h30 à 12h20</li>
```

La partie à distance porte des dates mais ni jour ni horaire : elle ne peut occuper aucune case d'un horaire.

## Décision

- `core::Mode` gagne une variante `Hybrid`, sérialisée `"hybrid"`, et `parse_mode` accepte « Hybride ».
  Le mode est conservé au lieu d'être ramené à `in-person` : un cours hybride se déroule autrement, l'étudiant doit le voir, et l'aplatir perdrait l'information silencieusement.
- **Aucune règle nouvelle pour ignorer la plage à distance** : `parse_slot` retourne déjà `Ok(None)` quand « Journée: » ou « Horaire: » manque.
  La moitié en ligne se retire donc d'elle-même, et une section hybride ne rend que ses rencontres en classe.
- Un test au niveau de la section (`a_hybrid_section_keeps_only_its_in_class_meetings`) épingle cette composition, pour qu'un remaniement futur de `parse_slot` ne puisse pas la casser en silence.
- GEX-3100 rejoint les fixtures gelées.

## Alternatives rejetées

- **Traiter « Hybride » comme `in-person`** : le solveur s'en accommoderait, mais l'UI ne pourrait plus distinguer un cours hybride d'un cours en classe.
- **Filtrer explicitement les plages « Type: Sur Internet »** : redondant — c'est l'absence d'horaire qui décide, et un filtre sur le libellé casserait à la première variante d'orthographe.
  Le mode se lit dans l'en-tête, pas dans « Type: » (ADR `2026-07-extraction-html-de-la-page-cours` § 2), et cette règle reste vraie.

## Effet mesuré

Sur GEX + GCI, les 24 anomalies « Malformed entry for mode » ont disparu et les sections concernées sont revenues dans les snapshots avec leur horaire en classe.
