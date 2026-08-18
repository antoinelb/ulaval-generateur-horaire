# L'interface prend la racine du site Pages

Date : 2026-08-17

## Contexte

L'ADR `2026-08-ci-et-publication-sur-github-pages` réservait explicitement la racine : « Pas de page d'accueil : la vraie UI prendra la racine plus tard. »
Les jalons 3–9 sont livrés, mais l'interface ne tournait que localement par `make ui` (`dx serve`) : il n'existait aucune cible de build de production.

Deux obstacles techniques attendaient à la racine :

- Pages sert un **site de projet**, donc sous `https://antoinelb.github.io/ulaval-generateur-horaire/`.
  `asset!()` émet des chemins **absolus** (`/assets/cours-dxh….json`) et l'`index.html` généré charge `/wasm/ulaval-scheduler-ui.js` : tout aurait répondu 404 sous le sous-chemin.
- Le service worker (`crates/ui/assets/sw.js`, DEG-3) était enregistré par `asset!("/assets/sw.js")`.
  La portée d'un service worker est le répertoire d'où il est servi : sous `/assets/` il pouvait mettre les données en cache, jamais contrôler la page. Le hors-ligne était donc inopérant, et l'en-tête `Service-Worker-Allowed` qui contournerait la règle n'est pas configurable sur Pages.

## Décision

Une cible `make ui-build` produit le site dans `_ui/public/` (dx imbrique le bundle web sous `public/`) :

```
dx bundle --release --platform web --package ulaval-scheduler-ui \
	--base-path ulaval-generateur-horaire --out-dir _ui
cp crates/ui/assets/sw.js _ui/public/sw.js
```

- Le sous-chemin est passé **en ligne de commande**, pas dans `Dioxus.toml` : `dx serve` continue de servir à la racine en développement, sans configuration conditionnelle.
- Le service worker est copié **à côté de l'index** et `browser.rs` l'enregistre par l'URL **relative** `"sw.js"` : elle résout au répertoire de la page, donc la portée couvre toute l'application quel que soit le chemin de base. Sous `dx serve` le fichier est absent, l'enregistrement échoue silencieusement (best-effort, OBS-6) — le hors-ligne est une propriété du site déployé.
- Le job `deploy` de `ci.yml` recopie `_ui/public/` à la racine de `_site` ; `pkg/`, `data/` et `docs/` restent inchangés — ce sont les points d'accès publics documentés, consommés par le dépôt frère `grille-de-cheminement-interactive`.

L'interface lit ses **propres** copies hachées sous `/assets/`, jamais `/data/` : les deux jeux coexistent (≈ 8,6 Mo dupliqués dans l'artefact, très en deçà de la limite de 1 Go).

Le partage par URL passe par le fragment (`browser.rs`), pas par des routes : aucun `404.html` n'est nécessaire pour le routage.

## Alternatives rejetées

- **`base_path` dans `Dioxus.toml`** : la valeur s'appliquerait aussi à `dx serve`, qui servirait alors l'application sous `/ulaval-generateur-horaire/` en développement — un écart dev/prod gratuit dans la mauvaise direction.
- **Domaine personnalisé (CNAME)** : donnerait la vraie racine et supprimerait le `--base-path`, mais coûte un domaine et une configuration DNS pour un site interne à l'Université.
- **Publier dans un dépôt `antoinelb.github.io`** : vraie racine, mais exige un jeton personnel et découple le site du code qui le produit — l'ADR `2026-08-documentation-mdbook-en-francais` a déjà rejeté une deuxième source de publication pour la même raison.
- **Garder le service worker en `asset!()`** : il mettrait en cache les données et le wasm mais pas la page ; un rechargement hors ligne échouerait quand même. Un hors-ligne partiel qui a l'air complet est pire que pas de hors-ligne.
