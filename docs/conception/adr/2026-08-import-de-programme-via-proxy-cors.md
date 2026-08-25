# Import d'un programme via un seul proxy CORS, corsproxy.io

## Contexte

L'application est entièrement statique et serverless : pas de backend, le navigateur charge le HTML directement.
`www.ulaval.ca` n'envoie pas d'en-tête CORS autorisant une lecture cross-origin, donc un `fetch` direct depuis l'UI échoue toujours.
L'étudiant doit pouvoir importer un programme absent des instantanés livrés (p. ex. B-GLO) sans qu'Antoine n'ajoute de backend ni ne republie de snapshot.

## Décision

- Un seul proxy, `corsproxy.io`, sans chaîne de secours : `browser::fetch_program_html` construit l'URL avec `crate::import::proxy_url`, qui encode l'URL cible à la main (pas `js_sys::encode_uri_component`, pour rester testable nativement) sous `https://corsproxy.io/?url=…`, et l'envoie avec `gloo-net` (déjà en dépendance).
- L'import est un **chemin non critique** (BLD-1) : sa panne ne casse rien d'autre dans l'app, et un programme déjà importé vit en `localStorage`, donc survit hors-ligne et à une panne du proxy (DEG-3) — voir ADR `2026-08-programmes-locaux-en-localstorage`.
- Les erreurs sont typées (`crate::import::ImportError` : `InvalidUrl`, `Proxy`, `NotFound`, `NotHtml`, `Parse`, `Preparatory`, `Cancelled`) et distinguent explicitement une panne du proxy, une page introuvable (`classify_response` traite 404/410 à part) et une réponse dont le type de contenu n'est pas `text/html` ou `application/xhtml+xml`. Chacune est présentée en cinq parties (ERR-1, `present::present_import_error`) : ce qui s'est passé, ce que l'app a fait, ce qui est affecté, quoi faire, un identifiant copiable — `Proxy` et `NotHtml` nomment `corsproxy.io` explicitement dans le texte, puisqu'un tiers intervient réellement dans la requête (TRU).
- L'annulation est réelle, pas cosmétique : `browser::ImportFetch` porte un `web_sys::AbortController`, et `abort()` interrompt effectivement le `fetch` en cours (comme `Solver::terminate` pour le worker) — abandonner la struct sans appeler `abort` ne l'annule pas.
- La mention du proxy est affichée sur la carte du programme local (`LocalMark.provenance`, « Importé le … via corsproxy.io. ») — la provenance d'un tiers dans le pipeline n'est jamais tue (TRU).
- Le tiroir `ProgramPicker` reste verrouillé pendant l'import et ne se referme jamais sur une erreur — l'étudiant voit le message et peut recoller une adresse sans perdre son point de départ.

## Alternatives rejetées

- **Un backend proxy maison** — l'architecture retenue pour tout le projet est statique et sans serveur (aucune base de données, aucun binaire à héberger) ; en ajouter un pour ce seul besoin romprait l'invariant du reste de l'app.
- **Plusieurs proxys en cascade** — complexité disproportionnée pour un chemin explicitement non critique (BLD-1) : une panne de corsproxy.io ne casse que l'import, jamais l'app déjà chargée.
- **Le collage de HTML en repli** — évite le proxy entièrement mais demande à l'étudiant de savoir « Enregistrer sous » une page web puis coller son contenu ; hors périmètre du plan (item Out of scope).
