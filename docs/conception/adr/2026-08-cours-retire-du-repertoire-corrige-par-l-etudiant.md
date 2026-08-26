# Un cours retiré du répertoire se corrige côté étudiant, pas dans le fichier manuel

## Contexte

ECN-2901 « Analyse économique en ingénierie » a été réussi (H25, relevé réel) puis retiré du répertoire : il n'est ni dans `data/cours.json` ni dans `data/cours.manuel.json`.
L'import Capsule le classe donc `NotInCatalogue` et ne l'épingle pas (ADR `2026-08-sigles-inconnus-du-releve-ignores`), et GEX-3333 — dont le préalable est `(ECN-2901 OU ECN-4901)`, identique au répertoire et à la correction manuelle — reste le seul refus du relevé.
`data/cours.manuel.json` sait porter un cours qu'aucune page ne décrit : l'ajouter là réglait le cas d'un coup.

## Décision

On ne l'ajoute pas (décision d'Antoine, 2026-08-26).
Un cours retiré du répertoire est un cas particulier de l'étudiant, pas une correction de catalogue : deux gestes de l'interface le couvrent déjà, chacun annulable.

- **Créditer ECN-4901 par entente** — il est aux règles du B-GEX, donc il a sa rangée et son bouton « créditer ». C'est la formulation exacte de la réalité : une entente reconnaît l'ancien cours pour le nouveau.
- **Corriger les préalables de GEX-3333** dans le champ « Préalables » (`plan.prereq_overrides`), qui prime sur toute correction partagée.

Le refus reste donc visible et nommé — « préalable manquant : ECN-2901 ou ECN-4901 » — au lieu d'être effacé pour tout le monde par une donnée que le répertoire ne porte plus.

## Alternatives rejetées

- **Ajouter ECN-2901 aux `courses` de `cours.manuel.json`** : le fichier manuel deviendrait le cimetière de tout cours retiré depuis que des étudiants en tiennent un — une liste sans fin, à maintenir à la main, pour des cas que l'interface règle déjà en deux clics.
- **Exempter des préalables tout ce qui vient du relevé** : déjà rejeté le 2026-08-26 (ADR `2026-08-concomitance-ouverte-par-le-releve`) — masque les vraies incohérences de données au lieu de les nommer.
