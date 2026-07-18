# Catalogue : l'union des facettes fait foi, bannière ignorée

Date : 2026-07-18

## Contexte

Le premier run partitionné a récolté 10 224 cours uniques quand la bannière en annonce 10 235.
L'enquête a montré que la bannière disait vrai : 11 cours réels (CLI-6000/6001/6501/6502/6503, EXD-8901, LAU-1011, MOB-1ADM/1FQT/6ADM/6SAD) n'appartiennent à aucune facette matière — le widget omet 4 matières, et 10 224 + 11 = 10 235 exactement.
Les facettes cycle sont trouées pareillement (somme 10 207).
C'est un bug du site, pas du scraper.

## Décision

- Le catalogue est **l'union des facettes matières** ; le widget qui omet des cours est un bug de l'ULaval que le scraper ne contourne pas.
- La bannière (`total_results` de la page 0 non filtrée) n'est **pas utilisée** : ni balayage complémentaire, ni réconciliation globale, ni avertissement (elle sert encore la sortie rapide « tout tient sur la page 0 »).
- La complétude garantie se limite à « chaque partition est complète pour sa requête » (réconciliation par partition inchangée) ; le filet global reste le garde « ≥ 90 % du compte précédent » prévu au plan.
- Le `total_results` du résultat combiné est le compte unique récolté.

## Alternatives rejetées

- **Balayage non filtré plafonné comme partition supplémentaire** : atteignait les 11 orphelins (~200 requêtes de plus), mais contourne un bug qui n'est pas le nôtre.
- **Réconciliation globale dure contre la bannière** : échouerait à chaque run tant que le widget est troué — le cron porterait l'échec de leur bug.
- **Avertissement consultatif sur l'écart** : l'écart est connu et accepté ; une alerte permanente devient du bruit.
