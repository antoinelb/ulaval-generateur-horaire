# « Réinitialiser » vise le document courant et supprime son étagère

## Contexte

Avec une étagère par (programme, millésime) (ADR `2026-08-instantane-de-plan-par-programme-et-millesime`), le bouton « Réinitialiser » doit dire ce qu'il remet à zéro — et sans purge d'étagère, rechoisir le programme ressusciterait la grille pré-reset : le bouton mentirait.

## Décision

- « Réinitialiser » ne touche que le **document courant** : l'`edit_plan` annulable existant (`*plan = Plan::default()`), plus `local_remove` de la clé d'étagère du programme courant. Les étagères des autres programmes survivent — les balayer d'un clic non annulable casserait la promesse « changer ne détruit rien ».
- La suppression d'étagère n'est pas dans l'historique : sans perte — un « Annuler » restaure le document vivant, retabletté au prochain départ.
- Le libellé du bouton reste « Réinitialiser » ; son infobulle et son toast précisent « ce programme ».
- Les fiches de cours manuels survivent toujours (ADR `2026-08-bouton-tout-reinitialiser`).

## Alternatives rejetées

- **Tout réinitialiser, étagères comprises** : un clic non annulable qui détruit le travail tabletté d'autres programmes.
- **Garder l'étagère** : rechoisir le programme annulerait silencieusement le reset.
