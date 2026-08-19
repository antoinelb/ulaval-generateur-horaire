# Sélection de la concentration et du profil au panneau, solveur scopé

Date : 2026-08-19

## Contexte

En génie mécanique (B-GMC), une concentration est de fait obligatoire — l'une s'appelle « Cheminement sans concentration », un bloc ordinaire de 18 crédits, pas une sentinelle.
La version JS (`grille-de-cheminement-interactive`, ADR « deux menus concentration et profil » du 2026-08-13) offre déjà ce choix ; l'UI Dioxus n'avait aucun contrôle, alors que `ProgramChoice.concentration/profile`, `coverage_report` par portée et le lien de partage étaient déjà en place.
L'étudiant doit pouvoir changer de concentration sans rien recommencer, pour explorer chacune.
Décisions validées sur maquettes (canevas « Sélecteur de concentration », 2026-08-19).

## Décision

- **Deux menus façon `panel-knob`** (« Concentration » / option neutre « Aucune », « Profil » / « Aucun ») en haut du panneau, au-dessus du bloc organigramme — changeables à tout moment.
  Un menu dont la liste est vide n'est pas rendu (B-GEX n'a pas de concentrations) ; un programme sans l'un ni l'autre n'a pas la rangée (M-GEX).
- **Défaut expert-sûr (AIR LAY-3, parité JS)** : au choix du programme, la première concentration du millésime est présélectionnée ; jamais de profil imposé ; un « Aucune » explicite est respecté et persiste (il est distinct de « Cheminement sans concentration », qui porte des règles).
- **Changer ne vide rien** : seul le choix change (acte étiqueté, annulable) ; la grille placée, les électifs et les épinglages restent.
  Les ententes `c/…` (resp. `f/…`) attachées au bloc quitté sont retirées dans le même acte et annoncées — leur règle a changé de sens avec le bloc ; « Annuler » les restaure.
- **Le solveur voit la portée choisie** : `placement_intake` prend `concentration`/`profile` ; les obligatoires des blocs choisis entrent dans `course_list` (après ceux du programme), leurs règles alimentent le bassin d'injection des électifs forcés, et les crédits « en sus » (`en_sus_codes`) ne couvrent plus que le programme et les blocs choisis.
  Un titre inconnu est une erreur typée (`UnknownConcentration`/`UnknownProfile`), jamais avalée — mêmes mots que le rapport de couverture.
  La requête au worker (`PlaceQuery`) retransporte `concentration`/`profile` (champs déjà présents sur le fil, `serde(default)`).
- **Les ententes se résolvent dans le bloc choisi seulement** : `grant_target`/`grantable_rules` ne regardent plus toutes les concentrations — toutes celles du B-GMC ont une « Règle 1 », une entente ne doit jamais atterrir dans le bloc d'à côté.
- **L'en-tête nomme le choix entier** : « Titre (CODE version A26) — Concentration — Profil », comme le sous-titre de la version JS.

## Alternatives rejetées

- **Étape 2 du choix de programme** : la concentration serait figée derrière le bouton « changer », qui rouvre tout — l'exploration demandée devient un recommencement.
- **Menus dans l'en-tête** (comme la version JS) : le panneau porte déjà tous les réglages du cheminement, et l'en-tête est plein ; le choix vit à côté des sections qu'il fait apparaître.
- **Affichage seulement (parité JS stricte)** : le bilan montrerait GMC-3351 manquant mais « Proposer » ne le placerait jamais — un verdict incohérent avec le bouton juste au-dessus.
- **Imposer un profil par défaut** : un profil est facultatif au répertoire ; l'imposer fausserait le bilan (même rejet que l'ADR JS).
- **Réinitialiser `View.expanded_rule` au changement** : une clé orpheline ne correspond à aucune section — rien d'ouvert, l'état se corrige au clic suivant ; rien à coder.
