# Fondation de tests navigateur (Playwright)

Date : 2026-08-30

## Contexte

Le dépôt n'avait aucun test navigateur.
Trois personas pilotées à la main le 2026-08-30 (`docs/ux/rapport-etudiante-2026-08-30.md`, `-cegep-`, `-directeur-gci-`) ont trouvé de vrais défauts — bandeaux qui recouvrent « Exporter ▾ » et les colonnes Jeudi/Vendredi, panneau qui saute à l'épinglage, gel inexpliqué de tout un horizon — et **rien n'en est resté d'exécutable**.
Chaque rapport se termine par « Erreur console : aucune », vérifié à la main, une fois.

Ce que les tests Rust ne peuvent pas voir, par construction : le rendu et les gestes.
Le solveur ne connaît que « ce cours est épinglé en session N » ; il ne sait pas s'il y est arrivé par un glissement, si le panneau a sauté ensuite, ni si un bandeau recouvre le bouton suivant.
La règle 10 d'AIR — « tout incident produit un test de régression » — n'avait donc pas de support pour cette moitié de l'interface.

Le dépôt voisin `../grille-de-cheminement-interactive` a déjà cette fondation.
On en reprend les **idées** (console propre en fixture automatique, tolérance par test, glissement synthétique, specs nommées en français) et rien de son code : ce dépôt-ci n'est plus une cible de compatibilité (ADR `2026-08-surface-javascript-plus-une-contrainte`).

## Décision

### Une suite Playwright sous `tests/e2e/`, cible `make e2e`, job CI qui l'appelle

`make e2e` dépend de `node_modules` et de `ui-build`, installe le navigateur puis lance `npx playwright test`.
La CI ajoute un job `e2e` qui installe les outils (Node, `dioxus-cli`, `wasm-pack`, les bibliothèques système du navigateur) puis appelle `make e2e` — aucune commande de vérification n'est recopiée (ADR `2026-08-makefile-definition-unique-de-la-ci`).
`deploy` attend maintenant `static`, `test` **et** `e2e`.

C'est la première dépendance npm du dépôt. Elle ne touche pas le produit : `package.json` ne sert qu'à Playwright, et rien de ce qui est publié n'en dépend.

### L'application testée est le bundle de production, servi sous son préfixe

`make ui-build` construit avec `--base-path ulaval-generateur-horaire`, et `asset!()` émet des URL **absolues** : servir `_ui/public` à la racine donne 404 sur chaque asset et une page nue, sans autre symptôme.
`tests/e2e/aides/serveur.mjs` (≈ 80 lignes, `node:http`, zéro dépendance) sert donc `_ui/public` sous `/ulaval-generateur-horaire/`, refuse tout ce qui est hors du préfixe en le disant, meurt avec la commande à lancer si le bundle manque, et porte le type MIME `application/wasm` sans lequel le module ne s'instancie pas.

**Alternative évaluée et rejetée : viser `dx serve` sur le port 8000.**
Plus simple à câbler — il sert déjà à la racine — mais trois choses l'interdisent :

1. le service worker n'existe **que** dans le bundle : « Under `dx serve` it is absent and nothing registers » (`crates/ui/assets/sw.js`). Tout le chemin de lecture hors ligne (AIR DEG-3) serait hors de portée de la suite ;
2. le serveur de développement injecte son client de rechargement à chaud et son bandeau « Your app is being rebuilt » — la persona cégep l'a vu passer au milieu de ses essais, et il rendrait l'assertion de console propre ingouvernable ;
3. un artefact qui se reconstruit sous les pieds du test n'est pas un sujet de test : ce qu'on veut vérifier est ce que la CI publie, à l'octet près.

### La console propre est une assertion, pas une inspection

`tests/e2e/aides/console-propre.js` exporte `test`/`expect` ; sa fixture `consolePropre` est **automatique**, donc impossible à oublier, et échoue le test dès qu'une erreur console ou une exception non capturée survient — sur la page comme sur toute page ouverte par le contexte.

La raison est plus forte ici que dans le dépôt JS : l'interface est du Rust compilé en WASM.
Une panique dans un composant ne vide pas l'écran ; elle remonte au bord `wasm-bindgen` comme exception non capturée et **le DOM du dernier rendu reste affiché**.
La page a donc l'air vivante pendant que le module est mort, et chaque `expect` qui suit interroge un cadavre — la suite deviendrait verte au moment précis où elle devrait crier.

La tolérance se déclare **par test** (`test.use({ toleranceConsole: [/motif/] })`), jamais globalement.
Un motif global au 404 masquerait aussi bien un 404 volontaire que le 404 d'un asset manquant — c'est-à-dire précisément le mode de défaillance du service sous préfixe décrit plus haut.

### Le glisser-déposer est prouvé, pas simulé

Le glisser-déposer HTML5 ne se déclenche pas par de vrais mouvements de souris sous Playwright : `mouse.down/move/up` ne produit aucun `DragEvent`.
`tests/e2e/aides/glisser-deposer.js` les dispatche à la main avec un `DataTransfer` partagé, et **renvoie toujours deux mesures** que les specs assertent :

- `dragoverPrevenu` — un navigateur ne délivre `drop` que si `dragover` a appelé `preventDefault()`. Sans cette lecture, un déposé synthétique « réussit » toujours, y compris là où le vrai navigateur refuserait. C'est aussi la seule preuve du refus d'une carte de session dont la saison n'offre pas le cours : l'aide ne délivre alors pas de `drop`, comme le navigateur ;
- `charge` — le jeton `text/plain` écrit par `dragstart`. Firefox refuse de porter un glissement dont le `DataTransfer` est vide ; si le `set_data` de `RibbonCode` disparaissait, Chromium continuerait de passer et Firefox casserait en silence.

### Un défaut connu se fixe avec `test.fail()`, jamais en assouplissant l'assertion

L'occlusion d'« Exporter ▾ » et des colonnes Jeudi/Vendredi par la pile d'avis est reproduite ici (mesurée par `elementFromPoint`, aux deux tailles de fenêtre) et **non corrigée** : c'est une décision de mise en page qui appartient à Antoine.
La spec l'énonce donc telle qu'elle doit être, sous `test.fail()` : la suite reste verte tant que le défaut est là, et devient rouge (« unexpected success ») le jour où il est corrigé — ce qui force à retirer l'annotation plutôt qu'à oublier le test.

### Attendre le solveur, jamais l'horloge

Le solveur tourne dans un Web Worker derrière une temporisation de 500 ms (`crate::solve::RECALC_DEBOUNCE_MS`).
Entre le geste et l'apparition de `.status-running`, l'écran est immobile et muet — mesuré à ~840 ms après un choix de programme.
Au premier jet, six specs se sont exécutées sur un ruban vide et ont échoué pour cette seule raison : leur attente avait lu deux fois « rien ne bouge » dans cette fenêtre aveugle et conclu que le calcul était fini avant qu'il ait commencé.
`attendreSolveur` exige donc **cinq lectures consécutives identiques espacées de 400 ms** — 1,6 s de calme continu, trois fois la temporisation — et les appelants attendent d'abord une condition *positive* (le ruban porte des sigles), parce que huit cartes de session vides existent avant même qu'un programme soit choisi.

## Retrait des `tests/user_stories/`

Les 78 fichiers sont supprimés (décision d'Antoine).
Ils décrivaient une interface qui n'existe plus — sélecteurs `#input-fichier-cheminement`, `.dropped-tile`, le flux CSV que l'ADR `2026-08-retrait-de-l-aller-retour-json-du-cheminement` a retiré — et aucun code ne les lisait : ni test, ni build, ni CI.
Une spécification que rien n'exécute et que personne ne relit dérive en silence, et une user story fausse coûte plus qu'une absente : plusieurs ADR les citent déjà comme des sources dont la référence était brisée (`2026-08-couleurs-derivees-de-la-matiere`).

Les specs de `tests/e2e/` deviennent la seule spécification exécutable de l'interface.
Les user stories restent dans l'historique git pour les ADR qui les citent.

## Alternatives rejetées

- **Ne rien tester au navigateur, s'appuyer sur les scénarios Rust natifs.** Ils couvrent la logique d'état, pas le rendu ni les gestes : aucun d'eux ne peut voir un bandeau qui recouvre un bouton, un panneau qui saute, ni un `dragover` qui ne prévient pas.
- **`dx serve` comme cible.** Voir plus haut : pas de service worker, bruit de console, artefact mouvant.
- **Servir `_ui/public` à la racine avec un serveur statique tout fait.** C'est le piège : 404 silencieux sur tous les assets, page nue, aucun message.
- **Une tolérance de console globale par motif d'URL.** Elle rendrait invisible exactement le mode de défaillance ci-dessus.
- **Un lien symbolique `ulaval-generateur-horaire → _ui/public` servi par `python3 -m http.server`.** Zéro ligne de JS, mais aucun diagnostic quand le bundle manque — et c'est là que le temps se perd.
- **Corriger l'occlusion des bandeaux pour rendre la spec verte.** Hors mandat : la mise en page des avis est une décision d'interface, pas un détail de test. `test.fail()` la garde visible sans la trancher à la place d'Antoine.
- **Firefox dans la matrice.** Le jeton `text/plain` que Firefox exige est asserté directement (`charge`), ce qui couvre le risque connu sans doubler le temps de CI. À rouvrir si un défaut propre à Firefox apparaît.
