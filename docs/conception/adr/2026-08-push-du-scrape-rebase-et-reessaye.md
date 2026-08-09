# Le push du scrape se rebase et réessaye, et échoue sur conflit

**Date :** 2026-08-09
**Statut :** accepté (décision Antoine).

## Contexte

Le job `scrape` fait son `checkout` au début, scrape pendant ~20 minutes, puis commite et pousse.
Son commit est donc bâti sur un `main` vieux de vingt minutes : un commit humain arrivé entre-temps fait rejeter le push en non-fast-forward, et l'exécution complète — vingt minutes à ~10 req/s — est perdue.

```
 ! [rejected]        main -> main (fetch first)
```

Le `concurrency: group: scrape` déjà en place ne protège que contre deux scrapes simultanés ; il n'a aucun effet sur un push humain.

Le commit du scrape ne touche que des fichiers **générés** — `data/catalogue.json`, `data/cours.json`, `data/programmes/{code}-{semestre}.json` — puisque `data/cache/` et les `data/*_errors.log` sont gitignorés et que les `*.manuel.json` ne sont jamais écrits par le scraper.
Le rebaser sur du travail humain est donc, en pratique, toujours sans conflit.

## Décision

- Le push est tenté, et sur rejet l'étape fait `git pull --rebase origin main` puis réessaye — boucle `for` **bornée à trois tentatives**, jamais un `while`.
  Une seule reprise suffirait (la fenêtre entre le rebase et le push suivant est de l'ordre de la seconde) ; trois est la marge.
- Le `checkout` passe à `fetch-depth: 0`. Le défaut de `actions/checkout` est un clone superficiel, sur lequel un rebase n'a pas de base de fusion garantie dès que l'historique distant a bougé autrement qu'en ajout simple. Le dépôt fait 4,5 Mo pour 58 commits : le clone complet ne coûte rien.
- **Un conflit de rebase avorte l'étape**, il n'est pas résolu automatiquement. Le `git pull --rebase` est laissé nu, donc le `bash -e` de GitHub Actions tue l'étape dès la première tentative ; le conflit reste dans les logs et le courriel d'échec de GitHub sert de notification, comme pour les anomalies du scraper.
- Le push est testé par `if git push`, pas par `git push && …` : sous `errexit`, l'élément gauche d'une liste `&&` est exempté de l'arrêt *et* l'échec de la liste entière ne le déclenche pas non plus, ce qui rendrait le contrôle de flux illisible.
- Le cas « rien à commiter » sort tôt (`exit 0`) au lieu d'un `else`, pour garder la boucle au premier niveau d'indentation.

## Alternatives rejetées

- **`-X theirs` sur le rebase (le scrape gagne)** : cohérent en apparence — le prochain scrape écraserait l'édition manuelle de toute façon — mais la résolution se fait hunk par hunk. Fusionner deux réécritures complètes d'un même JSON peut produire un fichier syntaxiquement valide et sémantiquement faux, publié tel quel sur Pages. Un échec visible vaut mieux qu'un snapshot silencieusement incohérent.
- **`git push --force-with-lease`** : écraserait le commit humain, exactement ce qu'on cherche à préserver.
- **Pousser sur une branche et ouvrir une PR** : demande une intervention manuelle à chaque scrape nocturne, alors que le contenu est généré et sans revue possible.
- **Une action tierce de commit automatique** : une dépendance de plus pour ce que six lignes de bash font, et il faudrait quand même choisir la politique de conflit.
