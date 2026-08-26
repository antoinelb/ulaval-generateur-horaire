# Une seule recherche, au pied du panneau, sans filtre horaire

Date : 2026-08-26

## Contexte

Le panneau ouvrait sur deux commandes et se fermait sur une troisième : le champ de recherche et la case « Seulement les cours qui rentrent dans l'horaire affiché » en tête, le champ « Ajouter par code… » tout en bas, sous « Cours absent du catalogue ? ».
Les trois servaient à la même chose — mettre un cours dans le plan — et les deux premières précédaient le contenu qu'elles filtraient.

## Décisions

- **La case « Seulement les cours qui rentrent dans l'horaire affiché » disparaît.**
  Elle masquait à tort : un cours écarté pour conflit reste choisissable, le solveur pouvant le placer ailleurs dans l'horizon, alors que la case ne jugeait que la session affichée.
  Et elle faisait doublon : chaque rangée porte déjà sa pastille de faisabilité (`panel::quick_fit` contre la sonde `fit_probe` de la session affichée), qui hiérarchise l'information au lieu de la retrancher.
  `SearchScope` perd `only_fitting` **et** `session` — les deux seuls appelants écrivaient `session: only_fitting.then(…)`, donc le filtre par saison mourait avec la case — et `SearchResults` perd `masked_by_fit`, le compteur des cours cachés qu'il n'y a plus lieu d'annoncer.
  `fit_probe` et `quick_fit` restent : les pastilles s'en servent.
- **Le champ « Ajouter par code… » disparaît.**
  Taper le sigle dans la recherche donne le même acte — la bande de choix de la rangée appelle `validate_new_code` par le même chemin — avec le titre, les crédits et l'état du cours sous les yeux plutôt qu'un verdict hors de toute fiche.
  `validate_new_code` garde ses autres portes (bande de choix, formulaire manuel, import Capsule).
- **La recherche descend au pied du panneau**, entre les notes — l'exigence linguistique comprise — et « Cours absent du catalogue ? ».
  Les trois gestes d'ajout se suivent alors du plus courant au plus rare : les règles du programme, la recherche du catalogue, le cours que le catalogue ignore.
- **Les blocs de règles restent affichés pendant une recherche**, au lieu d'être remplacés par les résultats.
  C'est la conséquence obligée du déplacement : un champ posé sous des blocs qui s'effacent à la première lettre remonterait sous le curseur (LAY-1).
  Les résultats s'ouvrent donc *sous* le champ, et le `scroll_to_results` armé à la frappe les amène à l'écran comme avant.

## Alternatives rejetées

- **Garder la case en la faisant juger tout l'horizon** plutôt que la seule session affichée : elle cesserait de mentir, mais resterait le doublon d'une information que chaque rangée porte déjà, et masquerait encore.
- **Descendre le champ en gardant le remplacement des blocs** : le champ saute sous le curseur dès la première lettre.
- **Garder « Ajouter par code… » en le fusionnant visuellement avec la recherche** : un seul champ à deux comportements selon que la saisie ressemble ou non à un sigle — deux verdicts possibles pour la même frappe.
