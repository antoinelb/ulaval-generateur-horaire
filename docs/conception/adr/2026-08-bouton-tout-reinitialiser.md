# Bouton « Réinitialiser » : un clic, annulable, jamais de confirmation

Date : 2026-08-14 (révisé le jour même — la première version armait une confirmation à deux temps, refusée par Antoine : les règles d'interface (ACT-2) exigent l'annulabilité, pas un dialogue)

## Contexte

Demande d'Antoine (2026-08-14) : un bouton qui remet l'application à zéro.
ACT-2 : chaque mutation est réversible et étiquetée, aucune confirmation nulle part — la réinitialisation ne fait pas exception.

## Décisions

- Bouton « Réinitialiser » dans l'en-tête (`header::ResetButton`) : **un seul clic**, qui passe par `edit_plan` (étiquette « Réinitialisation ») — « Annuler » restaure l'organigramme entier. Un toast le rappelle : « Tout a été réinitialisé — « Annuler » restaure votre organigramme. »
- **Portée : le document.** Le `Plan` revient à `Plan::default()` et la vue à `View::default()` (la vue n'est jamais annulable, par conception). Le fragment d'URL est retiré (`strip_query`) et une recherche en vol est annulée d'abord, pour qu'une proposition tardive n'atterrisse pas dans le plan neuf.
- **Les fiches de cours manuels survivent** : elles prolongent le *catalogue*, pas le document (persistées à part, ADR `2026-07-contribution-de-cours-manuels`), et une annulation peut restaurer un plan qui les référence — les effacer rendrait l'annulation mensongère. Plus aucun placement ne les référence après la réinitialisation ; leur retrait individuel reste un besoin séparé s'il se présente.

## Alternatives rejetées

- Confirmation à deux temps + effacement total non annulable (première version) : violait ACT-2 ; l'historique et les fiches partaient avec, rendant toute récupération impossible.
- Étendre l'historique d'annulation aux fiches manuelles pour pouvoir les effacer aussi : chirurgie de `History` (chaque `edit_plan` devrait transporter les fiches) pour un cas marginal — à revoir seulement si le besoin d'effacer les fiches devient réel.
- `window.confirm()` : bloque le fil, hors du style de l'application, et une confirmation reste une confirmation.
