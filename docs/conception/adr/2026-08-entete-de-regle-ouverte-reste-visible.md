# L'en-tête d'une règle ouverte reste visible dans le défileur du panneau

Date : 2026-08-23

## Contexte

Une règle de 25 cours fait disparaître son bouton de fermeture bien avant la fin de sa liste.
Le retour au titre exigeait alors un long défilement inverse.

## Décision

Le bouton `.panel-rule-head` d'une carte ouverte utilise `position: sticky` dans `.panel`.
La carte ouverte cesse de clipper ce bouton, de sorte que la limite de la carte arrête naturellement l'adhérence.
Le bouton conserve `aria-expanded` et une hauteur minimale de 48 px.

## Alternatives rejetées

- **Ajouter un second bouton de fermeture** — deux commandes identiques compliqueraient l'ordre clavier et l'état accessible.
