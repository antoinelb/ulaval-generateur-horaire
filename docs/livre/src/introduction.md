# Introduction

Le générateur d'horaire ULaval aide les étudiantes et étudiants de l'Université Laval — d'abord ceux du baccalauréat en génie des eaux — à construire deux choses :

- **l'horaire hebdomadaire** d'une session : quels cours, quelles sections, quels conflits ;
- **l'organigramme** du programme : quels cours à quelles sessions, du début du bac à la diplomation, en respectant préalables, plafonds de crédits et règles du programme.

Le projet est entièrement statique : aucun serveur, aucune base de données.
Quatre morceaux sont publiés sur GitHub Pages, à la même origine :

| Emplacement | Contenu |
|---|---|
| [`/`](https://antoinelb.github.io/ulaval-generateur-horaire/) | l'application : le générateur d'horaire et de cheminement |
| [`/pkg`](https://antoinelb.github.io/ulaval-generateur-horaire/pkg/) | le module WASM (`ulaval_scheduler_wasm.js` + `.wasm` + `.d.ts`), gelé et servi au mieux : plus une surface suivie (ADR `2026-08-surface-javascript-plus-une-contrainte`) |
| [`/data`](https://antoinelb.github.io/ulaval-generateur-horaire/data/cours.json) | les snapshots de données (cours et programmes), rafraîchis par un cron de scraping |
| `/docs` | ce livre |

## À qui s'adresse ce livre

- **À qui veut comprendre ou modifier le code** : la première partie décrit l'architecture du workspace Rust.
- **À qui doit interpréter les données** : la deuxième partie explique le vocabulaire du domaine — sessions, options, préalables, règles — et les décisions qui le structurent.

## Conventions

Le domaine est français : on parle de *cours*, *cheminement*, *préalables*, *session*, *concentration*.
Le code et les clés JSON sont anglais : `title`, `credits`, `prerequisites`, `mandatory`, `rules`.
Une entrée non reconnue n'est jamais ignorée en silence : elle est soit refusée (champ inconnu en entrée), soit remontée telle quelle (`raw`) pour que l'étudiant en juge.
