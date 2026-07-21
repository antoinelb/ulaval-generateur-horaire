# Fan-out : le nombre de pages calculé est une borne supérieure, les pages « Aucun résultat » excédentaires sont tolérées

Date : 2026-07-18

## Contexte

Premier run sur le catalogue complet : 199 pages réelles contre 204 calculées par ⌈total / entrées de la page 0⌉.
La taille de page réelle du site dépasse le nombre d'entrées observé en page 0, donc le calcul sur-estime le nombre de pages ; les pages excédentaires renvoient la variante « Aucun résultat » (`total_results: None`), que l'ADR `2026-07-pagination-du-catalogue-par-comptage` traitait comme une dérive (`PageTotalDrift`) — le run échouait.

## Décision

- Le nombre de pages calculé est rétrogradé d'« exact » à **borne supérieure du fan-out**.
- Pendant le fan-out, une page `total_results: None` est acceptée comme « au-delà de la fin » **seulement si elle ne porte ni entrée ni anomalie** ; elle contribue une page vide.
- Si elle porte des entrées ou des anomalies → `PageTotalDrift { got: None }` : aucune entrée jetée silencieusement.
- La garantie de complétude reste la réconciliation de `combine_pages` (compte fusionné == `total` de la page 0), inchangée : une sous-estimation du nombre de pages reste une erreur bruyante (`TotalMismatch`).
- Asymétrie assumée : sur-estimer coûte quelques requêtes qui résolvent en pages vides ; sous-estimer est impossible à rater.

Amende `2026-07-pagination-du-catalogue-par-comptage` : la réconciliation ne *remplace* plus la sentinelle « Aucun résultat » — l'arithmétique dimensionne le fan-out, la sentinelle absorbe son imprécision, la réconciliation garantit la complétude.

## Alternatives rejetées

- **Conserver l'erreur sur tout `None` (décision précédente)** : échoue sur le catalogue réel.
- **Lire le nombre de pages exact dans le widget de pagination du site** : un sélecteur de plus exposé à la dérive, sans meilleure garantie que la réconciliation déjà en place.
- **S'arrêter à la première page « Aucun résultat » sans réconciliation** : retour à la sentinelle seule — rien ne prouverait la complétude des pages du milieu.
