# US-38 — Case « Scolarité préparatoire complétée »

**Persona** : Mathieu, qui arrive avec tous ses cours d'appoint faits et ne veut pas les voir encombrer sa grille.
**Intention** : déclarer d'un coup que la scolarité préparatoire est acquise.

## Préconditions

- Un programme dont le fichier contient une règle « Scolarité préparatoire ».
  Pour le B-GEX A26 : `BIO-0150, CHM-0150, CHM-0160, CHM-0170, MAT-0130, MAT-0150`.

## Scénario

1. Mathieu observe que la case est cochée par défaut et que la section est repliée.
2. Il décoche la case.
3. Il la recoche.

## Résultats attendus

- Cochée, la case précharge les sigles de la règle comme s'ils étaient réussis, et leurs crédits comme accumulés, pour toutes les colonnes.
- Cochée, la règle apparaît au bilan avec ses crédits comblés, mais reste exclue du total global.
- Décochée, les cours qui dépendent des cours d'appoint sont signalés (US-02).
- Chaque bascule relance la vérification complète.

## Repères pour le test e2e

- `#scolarite-completee` existe uniquement si le programme a la règle correspondante, et vaut `checked` au chargement.
- L'en-tête de section porte `collapsed` et le bloc suivant porte `hidden` par défaut.
- Un clic sur la case ne replie pas la section : la propagation est arrêtée.

## Variantes et cas limites

- **Écart connu** : `MAT-1900` exige `MAT-0130 ET MAT-0150 ET MAT-0260`, mais `MAT-0260` n'est pas dans la règle « Scolarité préparatoire » du B-GEX A26. Cocher la case ne suffit donc pas et `MAT-1900` reste signalé. Le correctif appartient au calcul de la règle, côté scraper.
- L'état de la case n'est pas sauvegardé dans le CSV ni restauré.
- Un programme sans scolarité préparatoire n'affiche ni case ni section; la vérification doit fonctionner sans elles.
- La reconnaissance repose sur le titre exact « Scolarité préparatoire » : les millésimes convertis ont été renommés pour cela.
