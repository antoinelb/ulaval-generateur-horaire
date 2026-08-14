# US-50 — Données indisponibles ou corrompues

**Persona** : Daniel, directeur de programme, qui ouvre l'application le lendemain d'un scrape échoué.
**Intention** : comprendre ce qui ne va pas plutôt que de voir une page muette.

Le mode de défaillance redouté n'est pas la panne : c'est la donnée périmée en silence.

## Préconditions

- L'application est servie par HTTP; les données sont chargées par `fetch` de chemins relatifs.

## Scénario

Simuler tour à tour, avec l'interception de requêtes de Playwright :

1. `data/cours.json` renvoie 404.
2. `data/cours.json` renvoie un JSON sans liste `courses`.
3. `data/programmes/index.json` renvoie 404.
4. `data/programmes/B-GEX-A26.json` renvoie 500.
5. `index-programmes.csv` renvoie 404.
6. `b-gex/cours/cours-hors-catalogue.csv` renvoie 404.

## Résultats attendus

- Cas 1 et 2 : un message d'erreur explicite est affiché dans le résumé du programme; l'application ne reste pas sur une page vide sans explication.
- Cas 3 : le menu des sessions d'admission reste vide, un avertissement est journalisé en console, et le panneau reste sans règles.
- Cas 4 : l'erreur de chargement du programme est rapportée, avec le code HTTP.
- Cas 5 : le menu des programmes reste vide et l'application retombe sur `b-gmc`.
- Cas 6 : l'absence du fichier hors catalogue est tolérée sans erreur; seuls les pseudo-cours manquent.

## Repères pour le test e2e

- `page.route()` pour renvoyer les codes voulus.
- `#programme-summary-text` contient le message d'erreur.
- Aucune exception non capturée ne remonte dans `page.on('pageerror')`.

## Variantes et cas limites

- Ouvrir `index.html` par `file://` casse tous les `fetch` : l'application exige un serveur HTTP, ce qui doit rester un message clair et non un écran blanc.
- Un `data/cours.json` sans aucune année d'offre ne doit pas invalider tout le catalogue : la référence retombe sur l'année courante.
- Un fichier de programme sans `credits_required` donne un total exigé de 0 : le bilan doit rester lisible.
- Une borne de règle aberrante (négative, ou au-delà de 255) est ramenée à une valeur acceptable plutôt que de faire échouer le chargement du programme entier.
