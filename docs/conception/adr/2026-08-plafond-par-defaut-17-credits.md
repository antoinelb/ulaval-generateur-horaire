# Le plafond de crédits par défaut passe à 17

## Contexte

Le défaut de 15 cr était calibré sur le cheminement de référence GEX.
Le B-GMC ne tient pas dessous : ~117 cr d'obligatoires et de règles sur 8 sessions dépassent 8 × 15 = 120 dès qu'un cours au choix s'ajoute, et son cheminement officiel ouvre lui-même à 16 cr — premier contact systématique « rien placé » (rapport étudiante-cegep 2026-08-19).

## Décision

`DEFAULT_CREDIT_CAP = 17` : 8 × 17 = 136 donne l'aisance à tous les bacs livrés.
Le plafond reste un réglage du panneau (bornes 3..30 inchangées), jamais un mur.

## Alternatives rejetées

- **Défauts par programme depuis les `*.manuel.json`** : exigerait de charger les cheminements types dans l'UI (exclus du bundle) pour ne régler qu'un chiffre ; l'escalade des étés (ADR `2026-08-escalade-etes-ouverts-dans-le-repli`) couvre le reste.
- **Garder 15 et nommer le levier dans le message** : le premier contact du B-GMC resterait une grille vide.
