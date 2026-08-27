# Les couleurs de l'organigramme sont attribuées par matière sur le document complet

Date : 2026-08-26

## Contexte

L'export de l'organigramme attribuait une teinte distincte à chaque cours et recommençait la roue dans chaque session.
Deux cours d'une même matière pouvaient donc avoir des couleurs différentes, et la couleur d'un cours changeait lorsqu'il était déplacé dans une autre session.
La matière est le préfixe du sigle avant le tiret, conformément au modèle du domaine.

## Décision

Cette décision s'applique uniquement à l'export de l'organigramme.
L'horaire hebdomadaire et son export imprimé conservent leurs couleurs par cours.

Toutes les matières représentées par une case du document sont extraites après l'ajout des cases synthétiques de stage, triées alphabétiquement et dédupliquées.
La roue de 360° est divisée par ce nombre de matières, et la teinte d'une matière vaut `rang / nombre × 360`.
Toutes les cases d'une même matière reçoivent cette teinte, quelle que soit leur session.
La clarté, le chroma et le lavis oklch du gabarit imprimé restent inchangés.

## Alternatives rejetées

- **Une roue par session** : la même matière changeait de couleur après un déplacement.
- **Une teinte par cours sur tout le document** : cette méthode ne regroupait pas visuellement les cours d'une même matière.
- **Toutes les matières du catalogue** : des matières absentes de l'organigramme consommeraient inutilement des segments de la roue et rapprocheraient les couleurs visibles.
