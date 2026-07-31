# Un cours du programme sans données d'offre est écarté par le harnais, bruyamment

**Date :** 2026-07-30
**Statut :** accepté (décision harnais CLI, en autonomie — à revalider avec Antoine) ; découvert sur données réelles : GCI-1011, obligatoire du bac GEX 2026, n'apparaissait dans **aucun** snapshot 2009–2026 (page de cours sans aucune section de session).
**Portée réduite par `2026-07-cours-sans-section-de-session-offert-automne-hiver`** : GCI-1011 a désormais un `Course` (automne+hiver, horaire inconnu) et se place ; le chemin « écarté » ne couvre plus que les codes absents de `data/cours.json` — les vrais trous de scrape.

## Contexte

`--program` tire les obligatoires du snapshot programme ; un code sans page de cours n'a d'offre dans aucun snapshot, donc aucun domaine.
Le laisser dans la requête prouverait « aucun cheminement faisable » — techniquement vrai sous les données, mais le trou est un artefact de scrape, pas un fait de cheminement — et l'utilisateur ne peut pas retirer un obligatoire venu du fichier.

## Décision

- L'entrée **explicite** (codes positionnels, `--passed`, `--pinned`) reste strictement validée : un code inconnu des snapshots est une erreur — c'est une frappe de l'utilisateur, la protection contre la typo prime.
- Un cours **dérivé du programme** absent de tous les snapshots est **écarté du placement** et listé en tête de sortie (« Sans données d'offre… ») — dégradation bruyante, jamais silencieuse ; il reste dans la sélection du rapport de couverture (l'étudiant compte le suivre).
- `core` ne change pas : `place` refuse toujours un `passed`/`pinned` sans `Course` ; la dégradation est une politique de présentation du harnais.

## Alternatives rejetées

- **Erreur dure uniforme** : rend `--program` inutilisable sur le bac GEX réel d'aujourd'hui — le harnais échouerait sur son cas de démonstration principal.
- **Présumer une offre (placer quand même)** : invente des données d'inscription ; contraire à « jamais inventé ».
- **Écarter aussi les codes explicites inconnus** : une typo continuerait en silence relatif ; l'entrée tapée mérite la validation stricte.
