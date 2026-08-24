# Le panneau groupe les règles et les progressions par portée

Date : 2026-08-23

## Contexte

Le panneau fusionnait tous les cours obligatoires dans une seule carte et alignait ensuite toutes les règles sans nommer leur portée.
Une concentration ou un profil portant `credits_required` n'avait pas de progression fiable.

## Décision

Le panneau rend dans l'ordre `Programme`, `Concentration — …`, puis `Profil — …`.
Chaque portée garde ses propres obligatoires et omet la carte lorsque leur total vaut zéro.
La progression agrège l'union des obligatoires satisfaits et des cours comptés par les règles, déduplique les sigles et plafonne au total exigé.
Un crédit introuvable produit `—/Y cr` avec le sigle manquant.

## Alternatives rejetées

- **Additionner les badges de règles** — un même cours serait compté plusieurs fois.
- **Traiter un crédit absent comme zéro** — le panneau afficherait une précision qu'il ne possède pas.
