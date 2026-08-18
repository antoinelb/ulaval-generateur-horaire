# Des corrections de préalables qui valent pour tous les millésimes

Date : 2026-08-17

## Contexte

`2026-08-correction-des-prealables-par-millesime` n'a créé qu'un seul endroit où corriger des préalables à la main : `vintages.{millésime}.prerequisites`, dans `data/cours.manuel.json`.
Ce cadre suppose que la correction *dépend* de l'admission — l'étudiante reste régie par la version sous laquelle elle est entrée.

Toutes les corrections ne sont pas de cette nature.
GEX-3333 et GEX-3335 (« Projet intégrateur en génie des eaux I/II ») exigent `GCI-1011` selon le répertoire, alors que `GCI-1009` est également accepté.
Le répertoire se trompe, tout simplement ; l'erreur ne date d'aucun millésime et ne se corrigera pas en changeant d'année d'admission.
Écrire la même correction sous chacun des dix millésimes du B-GMC serait dix occasions de diverger.

Rien n'existait non plus pour corriger un préalable **avant** qu'un programme soit choisi : `effective_overrides` ne consultait `vintages` qu'à partir du millésime du plan, vide tant que le sélecteur n'a pas servi.

## Décision

`CourseManual` gagne un champ `prerequisites` de premier niveau, de la même forme que celui d'un `VintageOverlay` — code de cours → expression en texte brut, dans la grammaire du répertoire :

```json
{
  "courses": [ … ],
  "prerequisites": {
    "GEX-3333": "(ECN-2901 OU ECN-4901) ET (GCI-1009 OU GCI-1011) ET GMC-3009 ET  Crédits exigés : 72"
  },
  "vintages": { "A24": { "prerequisites": { … } } }
}
```

- **Le texte, jamais l'arbre.** L'arbre est reconstruit par `parse_prereq_tree`, donc il ne peut pas diverger de l'expression écrite. Une expression illisible est signalée (`OverrideNote::Unparsed`) et le cours garde ses préalables officiels ; elle ne retombe jamais en `Prerequisites::Raw`, qui préserve ce que l'université a écrit et n'est pas un dépotoir à coquilles.
- **Précédence du moins spécifique au plus spécifique** : sans millésime, puis le millésime d'admission, puis les corrections de l'étudiante elle-même. Le millésime l'emporte là où les deux nomment un cours.
- **La couche sans millésime s'applique toujours**, y compris quand aucun programme n'est choisi — un répertoire fautif ne dépend d'aucune admission.
- `CourseManual::overrides_for` devient la seule définition de cet ordre. `data::effective_overrides` la réimplémentait ; elle l'appelle désormais, et le `Snapshot` porte le `CourseManual` entier au lieu de sa seule table `vintages`. Le champ `shared_manual`, devenu exactement `manual.courses`, est supprimé.

## Alternatives rejetées

- **Une clé `"*"` dans `vintages`** : réutilise la table existante sans champ nouveau, mais « `*` » n'est pas un millésime et `malformed_vintages` devrait apprendre une exception — un cas particulier dans la fonction dont le rôle est justement de refuser les clés qui ne nomment aucune session.
- **Répéter la correction sous chaque millésime** : dix copies à maintenir pour le seul B-GMC, et rien avant qu'un programme soit choisi.
- **Accepter un objet `{text, official}`** plutôt qu'une chaîne, pour armer la garde de péremption de `PrereqOverride` : une correction « le répertoire se trompe » est censée cesser le jour où l'université corrige sa page, donc s'apercevoir que l'officiel a bougé aurait ici du sens — contrairement à une correction de millésime, qui est faite pour différer. Écarté pour l'instant : la forme demandée est la chaîne, et `official` reste disponible sans changer le format le jour où une correction en aura besoin.
- **Une entrée dans `courses`** : c'était la tentative initiale. `Course` exige `title`, `credits`, `cycle` et `seasons` ; une fiche partielle fait échouer la désérialisation, ce qui coûte **tout** le fichier manuel, pas la seule entrée fautive.
