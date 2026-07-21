# Contribution de cours manuels : fichier commité + issue GitHub préremplie

Date : 2026-07-19

## Contexte

`docs/project_plan.md:33` prévoit l'ajout manuel d'un cours avec son horaire (session à l'étranger, autre université).
Cet ajout est local (`localStorage`) et donc personnel : personne d'autre ne le voit.

Reste le cas où un cours doit être visible de tous alors qu'aucune source machine-lisible ne le porte — même situation que `cheminement_type` (`:78`).
Daniel doit pouvoir en proposer un lui-même, sans git, sans binaire à installer (`:83`).

Deux contraintes cadrent la solution :

- le scraper réécrit `data/cours/{session}.json` en entier à chaque cron, par remplacement atomique (`:93`) — une entrée ajoutée à la main y disparaît au run suivant ;
- il n'y a aucun backend (`:107`) : pas d'endpoint d'écriture, pas d'authentification, pas de stockage serveur.

## Décision

- **Fichier séparé et commité** : `data/cours/{session}.manuel.json`, même schéma que le snapshot scrapé, hors de portée du scraper par construction.
  `core` fusionne les deux au chargement ; les entrées manuelles portent `source: "manuel"`, les entrées scrapées `source: "ulaval"`.
- **En cas de collision de code, l'entrée scrapée gagne** : quand ULaval finit par publier le cours, la source officielle prime et l'entrée manuelle devient supprimable.
  La collision est signalée (affichée dans l'UI, pas seulement journalisée) plutôt que résolue en silence — invariant « ne jamais rien perdre silencieusement » (`:92`).
- **Le flag `source` est affiché** : l'UI marque visuellement un cours ajouté à la main, pour que personne ne prenne une donnée non vérifiée pour une donnée officielle.
- **Canal de contribution : lien d'issue GitHub préremplie.**
  Après un ajout manuel dans l'UI, un bouton « Proposer ce cours » ouvre `https://github.com/<repo>/issues/new?title=…&body=…` avec le JSON du cours déjà encodé dans le corps.
  C'est une simple ancre HTML : aucun appel réseau, aucun jeton, aucune permission.
- **La revue et le commit sont manuels** (Antoine) : le JSON de l'issue est collé dans le fichier, commité, et la CI redéploie le site statique.
  Le volume attendu (quelques cours par session) ne justifie aucune automatisation.

## Alternatives rejetées

- **Écrire dans `data/cours/{session}.json` directement** : effacé au prochain cron par le remplacement atomique.
- **Téléversement d'un fichier JSON par utilisateur** : demande un sélecteur de fichier, une validation, une histoire de versionnement et un hébergement — pour distribuer une donnée que le dépôt GitHub distribue déjà gratuitement (`:128` : commit → redéploiement).
  Surtout, cela remplace « aucun rituel de mise à jour » (`:83`) par un rituel de mise à jour pour chaque utilisateur.
- **Formulaire d'issue GitHub (`.github/ISSUE_TEMPLATE/*.yml`)** : ferait retaper à la main des champs que l'UI possède déjà sous forme structurée (code, titre, crédits, plages), avec le risque de saisie associé.
  Le lien prérempli transporte la donnée telle que construite. Un formulaire reste possible plus tard comme porte d'entrée sans passer par l'UI.
- **Contribution ouverte à tout utilisateur** : exigerait validation, modération et un canal d'écriture, donc un backend (`:107`).
- **Base de données ou service tiers pour les cours communautaires** : hors portée, et contredit « aucune persistance serveur » (`:91`).

## Hypothèse à valider

Le lien d'issue suppose que Daniel dispose d'un compte GitHub.
Si ce n'est pas le cas, la solution de repli est un bouton « Copier le JSON » vers le presse-papier, transmis par courriel — même coût côté code, seul le canal change.
