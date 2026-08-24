# Schéma du rapport de couverture des règles (`rules/`)

Date : 2026-07-29

## Contexte

Le vérificateur de règles (`rules.rs`, jalon 8) est l'API produit consommée par l'UI : par règle, « satisfait / à combler / candidats ».
Ses fixtures, écrites avant le code sur les données réelles des sept programmes, figent ce vocabulaire en JSON — comme `schedules/` a figé `schedule_report`.

## Décision

```json
{
  "program": { … },
  "concentration": "Géotechnique",
  "profile": null,
  "selection": ["MED-1100"],
  "courses": [ … ],
  "expected": {
    "mandatory": [ { "scope": "program", "satisfied": [], "missing": [ … ] } ],
    "rules": [
      { "scope": "program", "title": "Règle 1", "status": "satisfied",
        "counted": ["MED-1100"], "candidates": ["GMN-2901", "GMN-2902"] },
      { "scope": "program", "title": "Règle 2", "status": "incomplete",
        "counted": [], "elsewhere": ["MED-1100"], "missing": { "credits": 3 }, "candidates": [ … ] },
      { "scope": "program", "title": "Règle 5", "status": "reported", "raw": "…" }
    ],
    "language_requirement": { "status": "reported" }
  }
}
```

- `program` : le `core::Program` complet embarqué ; `concentration`/`profile` : titres optionnels — le rapport couvre toujours la portée `program`, plus les portées choisies (`scope` ∈ `program`/`concentration`/`profile`).
- `selection` : les codes choisis par l'étudiant ; `courses` : les `Course` complets des cours sélectionnés dont une règle à crédits doit compter les crédits.
- Statuts par règle : `satisfied` (`Count` : nombre de choisis dans la liste ≥ n ; `Credits` : somme dans `[min, max]`) ; `incomplete` avec `missing: {"count": k}` ou `{"credits": k}` — forme miroir de `Constraint` ; `reported` pour tout `Keyword` (`any`, `negotiated`), tout `Raw` et toute règle `constraint: None` — remontés avec leur `raw`, jamais inventés (`2026-07-contrainte-de-regle-optionnelle`, `2026-07-regles-negociees-reconnues`).
- Les listes de règles ont une sémantique d'**ensemble** : les doublons réels (règle 4 GEX : DDU-2000, ENT-1000, GGL-2601 chacun deux fois) dédoublonnent ; `counted` et `candidates` sont triés.
- `elsewhere` : les codes que la règle liste mais qu'une règle précédente de la même portée compte déjà ; omis quand vide, trié comme `counted`, exclu de `counted` et de `candidates`.
- `RuleCourses::Reference` est **résolue** vers la règle cible de la concentration nommée puis évaluée normalement ; une référence dont la cible est elle-même une référence est une erreur, pas une chasse.
- `candidates` = liste de la règle moins la sélection, **non filtrée** par `weekly::is_feasible` : le filtrage documenté suppose un contexte « horaire ouvert de la session visée » dont la forme d'entrée n'existe pas encore — cette famille épingle la couche comptable, la composition avec A sera épinglée par le harnais Rust.
- `language_requirement.status` : `satisfied` si le cours d'une des branches (francophone ou non) est dans la sélection, sinon `reported` — jamais « missing », car un score de test peut dispenser du cours et `core` ne le voit pas.

Reports consignés, non fixturés :

- double comptage d'un cours candidat à deux règles — tranché par `2026-08-un-cours-compte-dans-une-seule-regle-par-portee` : première règle de la portée qui liste le cours ;
- somme > `max` sur `Credits{min,max}` : violation ou excédent non compté n'est pas documenté — les fixtures restent ≤ max ;
- report des `credits_required` d'une concentration ou d'un profil sur le total (conception §7) — le rapport n'agrège pas de totaux.

## Conséquences

Quatorze fixtures gelées le 2026-07-29 sur les données réelles de cinq programmes (GEX, génie civil, génie mécanique, génie physique, génie industriel).
Deux faits que les données réelles ont livrés : la règle `constraint: None` que `docs/next_steps.md` prévoyait synthétique existe en vrai (génie mécanique, profil Passage intégré, `raw` tronqué « deuxième cycle suivante : ») ; une règle `{min: 0, max: …}` (génie industriel) est satisfaite à vide, 0 crédit ≥ 0.

## Alternatives rejetées

- **Un statut « missing » pour l'exigence linguistique** : affirmerait un manque que le score de test peut dispenser ; `reported` remonte sans trancher.
- **Compter les doublons des listes de règles** : deux occurrences du même code ne sont pas deux cours à réussir ; la page les duplique par mise en forme (sous-groupes thématiques).
- **Filtrer `candidates` par faisabilité dès cette famille** : figerait la forme du contexte horaire avant sa conception — la mauvaise moitié du contrat serait gelée en premier.
