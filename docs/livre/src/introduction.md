# Introduction

Le générateur d'horaire ULaval aide les étudiantes et étudiants de l'Université Laval — d'abord ceux du baccalauréat en génie des eaux — à construire deux choses :

- **l'horaire hebdomadaire** d'une session : quels cours, quelles sections, quels conflits ;
- **l'organigramme** du programme : quels cours à quelles sessions, du début du bac à la diplomation, en respectant préalables, plafonds de crédits et règles du programme.

Le projet est entièrement statique : aucun serveur, aucune base de données.
Trois morceaux sont publiés sur GitHub Pages, à la même origine :

| Emplacement | Contenu |
|---|---|
| [`/pkg`](https://antoinelb.github.io/ulaval-generateur-horaire/pkg/) | le module WASM (`ulaval_scheduler_wasm.js` + `.wasm` + `.d.ts`), importable par URL depuis n'importe quel HTML |
| [`/data`](https://antoinelb.github.io/ulaval-generateur-horaire/data/cours.json) | les snapshots de données (cours et programmes), rafraîchis par un cron de scraping |
| `/docs` | ce livre |

## À qui s'adresse ce livre

- **Au consommateur JavaScript** du module WASM : la première partie (guide) montre comment charger le module, appeler les quatre fonctions et interpréter leurs rapports.
- **À qui veut comprendre ou modifier le code** : la deuxième partie décrit l'architecture du workspace Rust.
- **À qui doit interpréter les données** : la troisième partie explique le vocabulaire du domaine — sessions, options, préalables, règles — et les décisions qui le structurent.

## Conventions

Le domaine est français : on parle de *cours*, *cheminement*, *préalables*, *session*, *concentration*.
Le code et les clés JSON sont anglais : `title`, `credits`, `prerequisites`, `mandatory`, `rules`.
Une entrée non reconnue n'est jamais ignorée en silence : elle est soit refusée (champ inconnu en entrée), soit remontée telle quelle (`raw`) pour que l'étudiant en juge.
