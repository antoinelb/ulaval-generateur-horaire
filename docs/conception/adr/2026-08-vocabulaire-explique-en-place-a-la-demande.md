# Le vocabulaire universitaire s'explique en place, à la demande

Date : 2026-08-30

## Contexte

Élodie, finissante au cégep, rapporte (2026-08-29) deux mots que rien n'explique nulle part dans l'application :

> Sur l'écran de choix de programme, chaque programme a un sélecteur de version (A26, H27…); une fois un programme choisi, l'en-tête affiche par exemple « 97/120 cr au bac (+9 cr en sus) ». Aucun texte explicatif visible nulle part sur ces deux points; il faut deviner ou connaître le jargon universitaire.

Les deux sont pourtant décisifs : la version choisie fixe les exigences qui s'appliquent (ADR `2026-08-millesime-de-programme-en-semestre`), et « en sus » explique pourquoi le total ne correspond pas à la somme des sessions.
Le « en sus » n'était accessible que par le `title` du compteur, c'est-à-dire au seul survol — sans effet au clavier ni au tactile.

## Décision

Deux explications, écrites en français clair dans `crate::present` (`VINTAGE_HELP`, `IN_ADDITION_HELP`), dépliées par un « ? » sur le patron déjà employé par « Charger depuis JSON » (ADR `2026-08-un-cheminement-par-fichier`) : bouton `aria-expanded`, texte rendu **en place**, refermable, jamais bloquant (LAY-4 — pas de visite guidée, pas d'accueil modal, pas d'infobulle qui masque la donnée).

- **Version** : le « ? » se range à côté de l'intitulé « Choisissez un programme… », et l'explication s'ouvre au-dessus de la liste des programmes, sans la recouvrir.
- **« +N cr en sus »** : le « ? » suit le compteur de l'en-tête; l'explication s'ouvre **sous** la bande d'en-tête, en pousse le contenu vers le bas et ne déplace ni ne recouvre rien (LAY-2). Il n'apparaît que lorsque le suffixe est là, puisqu'il explique ce suffixe.

Le texte du « en sus » nomme les stages parce que la donnée le permet : `credit_summary` ne verse dans `in_addition` que les cours des règles portant `credits_in_addition`, et cet indicateur n'est posé que sur la règle « Stages » promue depuis la prose des bacs de génie (ADR `2026-08-stage-obligatoire-en-prose-promu-en-regle`).

Rien d'autre ne change : ni défaut, ni comportement, ni jeu d'actions (LAY-3 — le support au novice est de l'explication *ajoutée*). Le `title` du compteur, qui décompose l'écart chiffré, reste tel quel : il répond à une autre question que « que veut dire en sus ».

## Alternatives rejetées

- **Une infobulle au survol** : sans équivalent au clavier ni au tactile, et elle recouvre la donnée qu'elle explique (INP-5, LAY-4).
- **Un accueil au premier lancement** : LAY-4 interdit la visite guidée et l'accueil modal; ils bloquent, et l'explication arrive au mauvais moment.
- **Un lien vers le répertoire de l'Université** : quitte l'application, ne survit pas hors ligne (DEG), et ne répond pas à la question posée à l'endroit où elle se pose.
- **Déplier l'explication de « en sus » à l'intérieur de la bande d'en-tête** : il aurait fallu la faire passer à la ligne (`flex-wrap`), ce qui change la disposition de tous ses éléments aux petites largeurs — un coût LAY-1 pour un texte optionnel.
