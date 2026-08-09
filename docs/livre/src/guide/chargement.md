# Charger le module

Le module est un paquet ES construit par `wasm-pack --target web` et servi sur GitHub Pages avec `Access-Control-Allow-Origin: *`.
Aucun bundler ni npm n'est nécessaire : on importe par URL, on initialise, puis on appelle.

```js
import init, {
  generate_schedule,
  verify_schedule,
  generate_organigramme,
  verify_organigramme,
} from "https://antoinelb.github.io/ulaval-generateur-horaire/pkg/ulaval_scheduler_wasm.js";

// télécharge et instancie le .wasm ; à faire une fois avant tout appel
await init();
```

## Charger les données

Les fonctions ne font aucune requête réseau : c'est l'appelant qui fournit les cours (et le programme) en argument.
Les snapshots sont servis à la même origine :

```js
const base = "https://antoinelb.github.io/ulaval-generateur-horaire";

// tous les cours, un seul fichier trié par code
const courses = await fetch(`${base}/data/cours.json`).then((r) => r.json());

// un programme, par code officiel et millésime (voir « Les données »)
const program = await fetch(`${base}/data/programmes/B-GEX-A26.json`).then(
  (r) => r.json(),
);
```

On peut passer le snapshot entier à chaque appel : les fonctions sélectionnent elles-mêmes les cours demandés (`codes`, `mandatory`, `electives`) et signalent ce qu'elles écartent.

## Types TypeScript

Le `.d.ts` du paquet déclare les quatre fonctions et toutes les formes d'entrée et de sortie (`ScheduleInput`, `ScheduleReport`, `OrganigrammeInput`, `OrganigrammeReport`, `Course`, `Program`, …).
Les déclarations sont dérivées des structures Rust elles-mêmes : elles suivent le code, pas une documentation parallèle.
Voir [Schémas d'entrée et de sortie](schemas.md) pour les subtilités (`null` explicites, clés absentes).
