# B retourne toutes les solutions faisables, pas seulement la première

**Date :** 2026-07-21
**Statut :** accepté ; révise la *sortie* de `2026-07-b-placement-par-satisfaction-fait-main` — tout le reste (placement seul, fait main, recherche complète, budget de nœuds, rejet jamais réparation) tient.

## Contexte

Décision produit (Antoine) : l'instance de B est plus petite que prévu — bac GEX : ~40 cours, 8 sessions, tronc largement fixé par les chaînes de préalables et l'offre par saison (conception §1.2) — au point que toutes les combinaisons devraient être vérifiables, pas seulement la première trouvée.
La recherche était déjà **complète** de toute façon : prouver « aucun cheminement faisable » exige de visiter tout l'espace élagué (conception §5.2).
S'arrêter à la première solution n'économisait donc que l'arrêt anticipé du cas faisable, en jetant les placements de rechange que la traversée avait déjà payés.

## Décision

- `place` retourne **toutes les solutions faisables trouvées**, dans l'ordre de la recherche — l'ordre de valeurs est inchangé (session du `cheminement_type` de référence d'abord), donc la première solution de la liste reste le placement « naturel » à proposer.
- Le budget de nœuds existant borne désormais aussi la **taille de l'ensemble retourné** (mémoire au navigateur, pas seulement le temps) ; les trois issues restent distinguées, jamais confondues :
  - recherche épuisée, ensemble non vide → l'ensemble est **complet** (toutes les solutions, prouvé) ;
  - recherche épuisée, ensemble vide → « aucun cheminement faisable » **prouvé** ;
  - budget atteint → ensemble **partiel**, signalé comme tel.
- Une mesure sur données réelles est ajoutée au plan (`docs/next_steps.md`, Phase 3) : compter les solutions du bac GEX complet *et* d'une liste partielle (tronc seul — le cas lâche où les placements se multiplient) avant tout raffinement.

## Alternatives rejetées

- **Première solution seulement (statu quo)** : le cas infaisable payait déjà la traversée complète ; les solutions de rechange étaient perdues pour rien.
- **Toutes les solutions sans budget** : une liste partielle laisse des cours lâches dont les placements explosent combinatoirement — mémoire non bornée en WASM ; le budget existant est le garde-fou, gratuit.
- **Dédoublonner par classes d'équivalence (électifs interchangeables) dès maintenant** : beaucoup de solutions ne différeront que par l'échange de deux cours équivalents, mais raffiner avant la mesure serait inventer le problème ; la donnée décide, comme le 1,21 option/cours l'a fait pour A.
