# Générateur d'horaire / planificateur de cheminement — conception

## Vue d'ensemble

Trois pièces conceptuelles, indépendantes de l'implémentation :

1. **Scraper** — extrait le catalogue, les horaires et les programmes des pages publiques de l'ULaval vers des fichiers JSON ; exécutable en ligne de commande ou depuis l'interface.
2. **Backend** — tous les calculs vivent ici : recherche, combinaisons de sections, conflits d'horaire, validation des préalables, suggestions, lancement/suivi du scraper.
   API REST JSON, sans état : chaque requête porte le cheminement, la réponse porte les résultats calculés.
   Progression du scraper diffusée par SSE.
3. **Frontend** — une SPA d'affichage : architecture Elm (modèle central, messages, vues), trois sections (scraper, sélection des cours, horaire hebdomadaire).
   Ligne rouge : **zéro règle métier côté client** — le modèle ne contient que la dernière réponse du backend et de l'état de pure vue (section active, élément en cours de drag).

Aucune base de données et **aucune persistance serveur** : le catalogue est en lecture seule (un JSON par session, chargé en mémoire au démarrage) ; le cheminement et les préférences de l'utilisateur vivent côté client.
Ça tourne tel quel sur un serveur externe ou en local comme application desktop.

L'interface et l'expérience utilisateur restent à explorer ; ce document identifie les fonctionnalités, pas leur agencement final.

## Hypothèse fondatrice (celle de Daniel)

Pour les sessions futures sans horaire publié, on réutilise l'horaire de la même saison la plus récente (A3 dans deux ans → dernier horaire d'automne connu).
Le scraper conserve donc un snapshot par saison, jamais écrasé aveuglément.

## Résultats du spike (2026-07-02)

Vérifié sur GCI-1007 (cas compliqué de référence, cours + laboratoires), la page catalogue, la page du bac GEX et trois cours à préalables (GAE-3008, GEX-4008, PLG-2104) :

- **Les pages cours sont rendues côté serveur** : session (« Automne 2026 »), NRC, capacité, enseignant, type (En classe/Laboratoire), dates, journée, plage horaire, pavillon, et les sections liées (A, B, …) avec leurs propres NRC sont tous dans le HTML d'un simple GET.
  Pas besoin de navigateur headless ; un parseur HTML suffit.
- **Le catalogue est un listing Drupal paginé** : ~205 pages × 50 cours ≈ 10 000 cours, liens rendus côté serveur, pagination par `?page=N`, facettes de filtrage par matière disponibles.
- **Volume** : un scrape complet ≈ 10 000 requêtes ; à une requête/seconde (politesse), ~3 h de roulage — d'où l'importance des filtres et de la reprise sur erreur.
- La matière étant le préfixe du code de cours (GCI-, GEX-), filtrer par matière ne demande aucune facette : on filtre les URL du listing.
- **La page programme est aussi rendue côté serveur et machine-lisible** : total de crédits exigés (« 120 crédits exigés »), bloc « Cours obligatoires » (code, titre, crédits), puis blocs « Règle N – \<contrainte\> parmi : » avec la liste de cours, parfois divisée en sous-groupes thématiques (Programmation, Langue et communication, Entrepreneuriat).
  Les contraintes observées : « Un cours parmi », « 3 crédits parmi », « 3 à 9 crédits parmi ».
- **Les préalables sont des expressions structurées** dont trois formes ont été observées :
  `((GAE-1004 ET GAE-2000) OU GCI-2009)` (booléen parenthésé), `BIO-1904 OU BIO-1925 OU PLG-1002 OU GEX-1000` (OU plat), `GEX, Crédits exigés : 60` (appartenance au programme + crédits accumulés).
- **Chaque page cours liste « Cette activité est contributoire dans : »** — la liste des programmes où le cours compte.
  Le mapping cours → programmes se construit donc depuis les pages cours elles-mêmes, sans scraper les ~400 pages programmes ; seuls les programmes dont on veut les règles ont besoin de leur page.
- Les pages cours exposent aussi les cours équivalents (ex. GEX-4008 ≡ GEX-7042), utiles plus tard pour la validation des préalables.

## Données

### `donnees/cours/{session}.json` (ex. `a2026.json`)

Pour chaque cours offert à cette session :

- code (`GCI-1007`), titre, crédits, cycle, matière
- préalables : texte brut + arbre parsé quand la grammaire le couvre (voir « Encodage automatique des règles et des préalables »)
- programmes contributoires (depuis « Cette activité est contributoire dans : ») et cours équivalents
- sections : NRC, type (en classe, laboratoire, forum…), capacité, enseignant, plages horaires (jour, heure début/fin, plage de dates, pavillon), sections liées et leur caractère obligatoire

### `donnees/programmes.json`

Scrapé des pages programmes (voir section suivante) : crédits exigés, cours obligatoires, règles.
Le cheminement type (quel cours dans quelle session, l'organigramme A1→H8) n'apparaît pas sur la page programme ; il reste encodé à la main pour GEX.

## Encodage automatique des règles et des préalables

### Règles de programme

La page programme suit une structure uniforme que le scraper traduit directement en JSON :

```json
{
  "baccalaureat-en-genie-des-eaux": {
    "titre": "Baccalauréat en génie des eaux",
    "credits_exiges": 120,
    "obligatoires": [{"code": "CHM-1903", "credits": 3}, {"code": "GCI-1000", "credits": 3}],
    "regles": [
      {
        "numero": 1,
        "contrainte": {"type": "cours", "nombre": 1},
        "cours": ["MED-1100", "GMN-2901", "GMN-2902"]
      },
      {
        "numero": 2,
        "contrainte": {"type": "credits", "min": 3, "max": 9},
        "cours": ["GCI-3101", "GEX-3500", "GEX-3502", "GGL-4001"]
      },
      {
        "numero": 4,
        "contrainte": {"type": "credits", "min": 3, "max": 3},
        "sous_groupes": [
          {"titre": "Programmation", "cours": ["IFT-4902"]},
          {"titre": "Langue et communication", "cours": ["ANL-2020", "FLS-2093", "FRN-1914"]}
        ]
      }
    ],
    "notes": ["Stage GEX-1580 obligatoire pour diplômer, crédits en sus du programme."],
    "cheminement_type": {"A1": ["CHM-1903", "MAT-1900", "GCI-1000"], "H2": ["IFT-1903"]}
  }
}
```

- L'en-tête « Règle N – \<contrainte\> parmi : » se normalise avec trois motifs : « Un cours » → `{type: cours, nombre: 1}`, « X crédits » → `{type: credits, min: X, max: X}`, « X à Y crédits » → `{type: credits, min: X, max: Y}`.
- Les sous-groupes thématiques deviennent `sous_groupes` ; les prescriptions en prose (stages, exigences d'anglais de la règle 4) sont conservées telles quelles dans `notes` et affichées, pas interprétées.
- `cheminement_type` est le seul champ maintenu à la main (absent de la page).
- Le scraper étant guidé par la structure et non par le contenu, il fonctionne pour n'importe quel programme ; GEX sert de cas de validation.
- Défense : toute règle dont l'en-tête ne correspond à aucun motif est conservée en texte brut et signalée dans la progression du scraper, jamais ignorée silencieusement.

### Préalables

Grammaire couvrant les trois formes observées :

```
expr        := terme (OU terme)*
terme       := facteur (ET facteur)*
facteur     := '(' expr ')' | CODE_COURS | exigence_credits
exigence_credits := PROGRAMME ', Crédits exigés : ' N
```

Exemples d'arbres produits :

```json
{"OU": [{"ET": ["GAE-1004", "GAE-2000"]}, "GCI-2009"]}
{"OU": ["BIO-1904", "BIO-1925", "PLG-1002", "GEX-1000"]}
{"credits_programme": {"programme": "GEX", "credits": 60}}
```

Défense : une expression hors grammaire est conservée en `{"brut": "…"}` — affichée à l'utilisateur, comptée dans le rapport du scraper, jamais bloquante.
La grammaire s'étendra au fil des cas réels rencontrés sur le catalogue (concomitants, etc.).

## Fonctionnalités

### Section 1 — scraper : configuration et progression

- Lancer le scraper depuis l'interface (et en ligne de commande pour l'automatisation).
- Filtres : tout le catalogue, certains programmes, ou certaines matières.
- Progression affichée en direct par SSE : cours traités / total, cours en erreur, session(s) découvertes.
- Reprise sur erreur et throttling (une requête/seconde) ; un scrape interrompu se poursuit sans tout refaire.
- Les snapshots existants restent servis pendant un scrape ; remplacement seulement à la fin (écriture atomique : fichier temporaire puis `rename`).

### Section 2 — sélection des cours (bac ou session)

- Recherche de cours reproduisant la fonctionnalité (pas l'interface) de la page ULaval, réduite à : matière, cycle, session, programme.
  « Baccalauréat en génie des eaux » coché par défaut.
- Organigramme du bac : colonnes par session (A1…H8 mappées sur des sessions réelles), cheminement type GEX pré-chargé, glisser-déposer des cours entre sessions, ajout depuis la recherche, retrait.
- Indicateurs par cours : offert ou non à cette session, conflit d'horaire, préalable non respecté (fonctionnalité ultérieure).
- Bouton « suggérer » (fonctionnalité ultérieure) : une fois les cours obligatoires placés, propose des cours pour combler les règles du programme (ex. règle 4 : 3 crédits parmi une liste) selon les sessions d'offre et les conflits.
- Chaque déplacement de cours envoie le cheminement au backend, qui recalcule les indicateurs et retourne l'état complet à afficher.

### Section 3 — horaire hebdomadaire d'une session

- Grille hebdomadaire classique (lundi–vendredi, 8h30–21h30) pour la session choisie.
- Choix automatique d'une combinaison de sections sans conflit (backtracking borné — une section de chaque type par cours, sections liées obligatoires incluses), calculé côté backend.
- S'il n'existe aucune combinaison valide, les plages en conflit sont identifiées dans la réponse et surlignées à l'affichage.
- Override manuel : forcer une section précise (NRC) ; le backend recalcule autour.

### Moteur (backend, transversal)

- Tous les calculs côté backend : recherche, combinaisons de sections, détection de conflits, validation des préalables, suggestions.
  Le backend est sans état : cheminement en entrée, résultats calculés en sortie.
- Parsing des expressions de préalables et validation de l'ordre des cours dans l'organigramme (fonctionnalité ultérieure).

### Persistance et reprise (transversal, tout côté client)

- Aucune persistance serveur.
- Reprise : filtres de recherche (le bac GEX coché reste coché), section active, session affichée et cheminement courant persistés en `localStorage` — l'utilisateur reprend exactement où il était.
- Aucun partage pour le moment.
  Si le besoin émerge : URL compacte encodant la différence avec le cheminement type (ex. `?p=gex&d=+PLG-2104@A5,-GEX-1000`) ou export/import JSON.

## Implémentation — deux propositions

Les fonctionnalités, les formats de données et la répartition backend/frontend ci-dessus sont identiques dans les deux cas ; seuls le langage et l'outillage changent.

### Proposition A — Python + JS vanilla (architecture Elm)

- **Scraper** : Python (httpx + parseur HTML), CLI + déclenchable par l'API.
- **Backend** : Starlette, API REST JSON + endpoint SSE pour la progression du scraper.
- **Frontend** : vanilla JS en architecture Elm, le pattern de Holmes — `initModel`/`update`/`view` par section, helpers réutilisés (`elements.js`, `listeners.js`), pas de framework, pas de build step.
- **Forces** : stack déjà maîtrisé (Holmes en production), estimation fiable, chaque jalon démontrable au rythme prévu.
- **Faiblesses** : pas de version desktop native (c'est le navigateur qui joue ce rôle) ; aucun apprentissage neuf.

| # | Jalon démontrable | Heures |
|---|---|---|
| 1 | Scraper CLI : cours GEX d'une session → JSON valide, GCI-1007 correct | 8–12 |
| 2 | Horaire d'une session : cours choisis dans une liste simple, grille, combinaison automatique, conflits | 10–14 |
| 3 | Recherche + organigramme drag-and-drop du bac GEX + reprise localStorage | 12–16 |
| 4 | Scraper depuis l'interface : filtres, progression SSE, reprise sur erreur | 6–8 |
| 5 | Scraper des pages programmes : obligatoires + règles en JSON, validé sur GEX | 5–8 |
| 6 | Préalables : parsing + validation de l'ordre | 6–10 |
| 7 | Suggestions selon les règles du programme | 6–10 |

**Cœur (1–4) : ≈ 36–50 h. Vision complète (1–7) : ≈ 53–78 h.**

### Proposition B — Dioxus fullstack (Rust)

- **Scraper** : Rust (reqwest + scraper + serde), CLI + déclenchable par l'API.
- **Backend** : server functions Dioxus (Axum), REST JSON + SSE.
- **Frontend** : Dioxus 0.7 — composants RSX, signaux ; **web (vrai DOM via WASM) et desktop (webview) depuis le même codebase**.
- **Forces** : un seul langage partout, typé de bout en bout ; livrable web *et* desktop natif ; l'opportunité d'apprentissage Rust web/desktop recherchée.
- **Faiblesses** : idiome React (composants/signaux), pas l'architecture Elm ; stack en apprentissage → estimation gonflée et moins fiable ; temps de compilation et tooling WASM dans la boucle de développement.
- Écarté : Iced — architecture Elm native séduisante, mais son rendu web est un canvas WGPU (pas de DOM, accessibilité faible) ; à réserver à un projet desktop-first.

Mêmes jalons, avec l'apprentissage intégré :

**Cœur (1–4) : ≈ 70–100 h. Vision complète (1–7) : ≈ 100–150 h.**

L'écart avec A est du temps d'apprentissage, pas du temps de produit ; à annoncer à Daniel comme tel (ou à absorber comme investissement personnel non facturé).

### Position mitoyenne possible

Rust au scraper seulement (jalon 1, borné et testable sur les HTML déjà téléchargés), le reste en proposition A : apprentissage réel, risque contenu, +4–8 h sur le jalon 1 uniquement.

## Progression

Chaque jalon est démontrable ; à ~10 h/semaine, environ un jalon par semaine en proposition A.
Le spike ayant confirmé que tout est rendu côté serveur (pages cours, catalogue et programmes), le principal risque d'estimation restant est la variété des cas particuliers du catalogue (stages, cours multi-sessions, formation à distance, formes de préalables non observées).

## Décisions prises (à contester au besoin)

- JSON par session, pas de SQLite — données en lecture seule, volume modeste même pour tout le catalogue.
- Sections choisies automatiquement avec override manuel — c'est le « ça monte l'horaire tout de suite » de Daniel sans enlever le contrôle.
- Règles et préalables scrapés automatiquement (structure uniforme confirmée) ; seul le cheminement type A1→H8 est encodé à la main, faute de source machine-lisible.
- Filtre « programme » de la recherche construit depuis les pages cours (« contributoire dans ») — pas besoin de scraper les ~400 pages programmes.
- Tous les calculs côté backend ; frontend en architecture Elm mais **sans aucune règle métier** — le modèle client est la dernière réponse serveur + l'état de vue.
- API REST JSON + SSE pour la progression du scraper ; pas de WebSocket.
- Aucune persistance serveur — reprise par `localStorage` ; aucun partage pour le moment (URL compacte ou export/import JSON si le besoin émerge).
- Parsing HTML simple (pas de navigateur headless) — confirmé par le spike.

## Questions ouvertes

- Choix entre les propositions A et B (ou la mitoyenne) — dépend de l'arbitrage apprentissage/heures annoncées.
- Agencement de l'interface (les trois sections) — à explorer une fois les fonctionnalités gelées.
