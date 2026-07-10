# Plan de livrable — générateur d'horaire GEX

Heures : proposition A (Python + JS vanilla) de `CONCEPTION.md`.
Si le développement se fait en Rust (proposition B), les heures d'apprentissage supplémentaires sont absorbées par Antoine et ne changent pas ce plan.

## 1. Requis obligatoires (documentés par Daniel, courriels de juin–juillet 2026)

Ce que Daniel a explicitement demandé :

- « un petit menu où on sélectionnerait les cours pour une session donnée et ça viendrait tout de suite monter l'horaire associé » ;
- « en prenant l'information disponible sur le site web de l'université » ;
- « en présumant que l'horaire ne changerait pas d'une année à l'autre » ;
- « Ce serait très cool qu'il puisse s'appliquer à l'ensemble du catalogue de cours de l'université » (souhait explicite).

| Fonctionnalité | Jalon | Heures |
|---|---|---:|
| Scraper des pages ULaval : cours d'une session → JSON, snapshots par saison réutilisés d'année en année (hypothèse de Daniel) | 1 | 8–12 |
| Sélection des cours d'une session dans un menu + horaire monté immédiatement : grille hebdomadaire, incluant nouveaux cours manuels (e.g. session étranger / autre université), combinaison automatique des sections, conflits surlignés | 2 | 10–14 |
| Application à l'ensemble du catalogue (~10 000 cours) : filtres par programme/matière, reprise sur erreur, throttling, progression en direct* | 4 | 6–8 |

**Sous-total requis : ≈ 24–34 h**

\* Le lancement du scraper depuis l'interface avec progression SSE est un enrichissement de conception regroupé dans ce jalon ; le séparer donnerait une fausse précision.

## 2. Fonctionnalités additionnelles (issues de la conception)

Ce qui a émergé du travail de conception au-delà des courriels :

| Fonctionnalité | Jalon | Heures |
|---|---|---:|
| Recherche de cours (matière, cycle, session, programme) + organigramme du bac (A1→H8) avec glisser-déposer entre sessions + reprise `localStorage` | 3 | 12–16 |
| Scraper des pages programmes : cours obligatoires et règles (« Règle N – X crédits parmi ») en JSON, validé sur GEX | 5 | 5–8 |
| Préalables : parsing des expressions (ET/OU, crédits exigés) + validation de l'ordre des cours dans l'organigramme | 6 | 6–10 |
| Bouton « suggérer » : proposer des cours comblant les règles du programme selon les sessions d'offre et les conflits | 7 | 6–10 |

**Sous-total additionnel : ≈ 29–44 h**

## Totaux

| Portée | Heures |
|---|---:|
| Requis de Daniel (jalons 1, 2, 4) | 24–34 |
| Vision complète (jalons 1–7) | 53–78 |

Chaque jalon est démontrable ; à ~10 h/semaine, environ un jalon par semaine.

## Questions ouvertes

- Agencement de l'interface (les trois sections : scraper, sélection, horaire) — à explorer une fois les fonctionnalités gelées.
- Hébergement : serveur externe ou exécution locale sur le poste de Daniel? Les deux fonctionnent (aucune persistance serveur), mais ça détermine qui lance le scraper et où vivent les snapshots.
- Cheminement type A1→H8 : encodé à la main pour GEX (absent des pages web) ; si d'autres programmes veulent l'organigramme pré-rempli, qui fournit leur cheminement type?
- Couverture des cas particuliers du catalogue (stages, cours multi-sessions, formation à distance, formes de préalables non observées) : quel niveau de couverture est exigé avant livraison? C'est le principal risque résiduel d'estimation.
