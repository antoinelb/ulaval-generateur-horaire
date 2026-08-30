# La bascule dit où va le travail, et pourquoi « Annuler » est éteint

Date : 2026-08-30

## Contexte

L'ADR `2026-08-reinitialiser-reste-dans-le-programme` avait déjà fait pousser à « changer » un ✓ nommant la façon de récupérer le cheminement tabletté (`present::shelved_note`).
Élodie (finissante au cégep, 2026-08-29) ne l'a pas vu, et décrit exactement ce que l'écran lui montre :

> « L'en-tête revient à « aucun programme choisi », les 8 sessions redeviennent « à planifier », « 0 cr cette session » s'affiche, et le bouton « ↶ Annuler Ctrl+Z » redevient grisé. Re-choisir « B-GCI » : le cheminement et la concentration reviennent exactement comme avant. L'écran donne tous les signaux d'une perte totale et irréversible alors que les données sont conservées par programme en arrière-plan. »

Le message existait donc, mais :

- il était un `AlertBody::Success`, effacé par la minuterie de 5 s pendant qu'elle regardait ailleurs (le bandeau, le ruban, la grille — tout ce qui venait de s'éteindre) ;
- il ne parlait que de l'étagère, et laissait sans réponse le signal le plus alarmant de l'écran : le « Annuler » grisé. Élodie relève l'incohérence elle-même — « « Réinitialiser » reste annulable par Ctrl+Z, contrairement à « changer », qui désactive Annuler alors même que les données ne sont pas perdues ».

## Décision

- **La bascule n'entre pas dans l'historique du plan.** L'ADR `2026-08-historique-par-document-vide-a-la-bascule` l'a tranché et rien ici ne le rouvre : un undo qui traverserait la bascule restaurerait le plan du programme A pendant que A est aussi sur l'étagère — deux copies divergentes du même document. « changer » remplace le document, il ne l'édite pas ; c'est la seconde porte, pas `edit_plan`.
- **C'est donc l'écran qui cesse de mentir**, sur les deux points :
  - `present::shelved_note` gagne une seconde phrase qui nomme la vraie cause du bouton éteint : « « Annuler » est éteint parce que l'historique appartient à chaque programme, pas parce que votre travail a été perdu. » ;
  - l'avis passe de `AlertBody::Success` à `AlertBody::Standing`, un ✓ de même apparence qu'**aucune minuterie n'efface**. ALR-4 n'autorise l'effacement automatique qu'à la priorité la plus basse ; « votre cheminement est conservé, voici le geste qui le ramène » est une consigne sur laquelle l'étudiante a encore à agir, pas une simple confirmation.
- **Le bouton lui-même dit pourquoi il est éteint** : `present::undo_title` / `present::redo_title` remplacent le « Rien à annuler » du bandeau par « Rien à annuler dans ce programme. L'historique repart à zéro à chaque changement de programme, mais le cheminement de chacun reste conservé. » La formulation vit dans le module pur, testée (AP-5).

## Alternatives rejetées

- **Faire survivre `History` à la bascule, ou enregistrer la bascule comme un acte d'`edit_plan`** : le fork étagère/pile déjà refusé le 2026-08-19, et un « Annuler » qui téléporterait l'étudiante vers un autre programme sans le dire.
- **Une pile d'historique par étagère** : de la mémoire et de la persistance pour un besoin que l'étagère couvre déjà à l'identique.
- **Ne dire la chose que dans l'infobulle du bouton** : une affordance au survol n'en est pas une (elle n'existe ni au clavier ni au tactile), et c'est précisément l'état que l'ADR du 2026-08-29 corrigeait déjà une première fois.
- **Un dialogue de confirmation avant de changer de programme** : ACT-2 l'interdit, et la bascule ne détruit rien — c'est justement ce que l'écran devait arriver à dire.
- **Garder le ✓ auto-effacé et allonger sa minuterie** : un seuil arbitraire de plus, qui perd toujours la course contre une étudiante qui lit l'en-tête avant les coins de l'écran.
