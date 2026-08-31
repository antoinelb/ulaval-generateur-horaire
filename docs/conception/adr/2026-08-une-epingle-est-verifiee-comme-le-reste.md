# Une épingle est vérifiée comme le reste

Date : 2026-08-30

**Statut :** accepté. **Amende** `2026-08-etoile-de-concomitance-au-parsing` (rien du parsing ne change ; c'est sa lecture côté interface qui est corrigée) et `2026-08-organigramme-en-continu-sans-bouton` (la fraîcheur du verdict automatique y gagne une condition).

## Contexte

Une étudiante en génie des eaux, le 2026-08-30 : dans B-GEX A26, épingler MAT-1910 dans la session A1 — celle qui porte déjà MAT-1900, son préalable strict — est accepté sans un mot, case « Permettre un préalable en concomitance » décochée, l'écran affichant « sans conflit ✓ ».
Reproduit deux fois, dont une après « Réinitialiser ».
Sa conclusion tenait en une phrase : « je ne peux plus être sûre qu'un cheminement marqué sans conflit ✓ est réellement valide dès que je touche moi-même à l'épinglage manuel ».

**Le noyau n'était pas en cause**, et la reproduction native l'a prouvé avant toute correction.
En rejouant la requête exacte de l'interface (`solve::verify_request`) à travers `wasm::protocol::handle`, sur les vraies données B-GEX A26 :

| requête | réponse |
| --- | --- |
| `verify` de la grille automatique (témoin) | 1 solution |
| `verify`, MAT-1910 épinglé dans la session de MAT-1900 | **0 solution**, `completion: "complete"` |
| `place`, même épingle | repli au mieux, `left_out: ["MAT-1910"]` |

`complete` sans solution est une infaisabilité **prouvée**, pas un budget dépassé : `precedence_admits` et `finalize` jugent une épingle comme n'importe quel cours, `value_ordered_domain` ne faisant que réduire son domaine à un singleton.
Le défaut était donc entièrement en aval, dans `crates/ui`, et il tenait en cinq points — quatre lus dans le code, un confirmé par lecture du protocole :

1. **L'acte d'épinglage ne validait rien.** `components::panel::place_course` écrivait `pinned_sessions` et `displayed_placement` sans un seul appel de préalable ; les deux glisser-déposer du ruban l'appelaient directement.
2. **Le garde-fou de la bande de puces s'auto-annulait.** `take_verdict` sortait par `if choice != Choice::Not { return Some(None) }`, et `panel::choice` renvoie `Choice::Auto` dès qu'un cours est obligatoire — ce que MAT-1910 est en B-GEX.
3. **Les deux contrôles statiques étaient aveugles aux sessions.** `solve::validate_new_code` construisait son ensemble `held` à partir de tout `displayed_placement`, sans ordre, et passait un `same_session` **vide** : un préalable posé la même session — ou trois sessions plus tard — comptait comme acquis.
4. **Quand le solveur refusait, l'explication était supprimée.** `adoption_regressions` filtre le cours hors de `left_out` (puisque `place_course` l'avait assis dans la grille), donc la boucle qui produit `left_out_line` — la seule ligne qui nomme le préalable — ne l'atteignait jamais ; restait un « Proposition ignorée » générique, que la mémoire des rejets d'`AlertStack` peut rendre muet pour de bon.
5. **Un verdict périmé était accepté comme frais.** À la réponse d'un `verify`, `verification_stale` retombait à `false` sans vérifier que le plan n'avait pas bougé entre l'envoi et la réponse ; `Running` ne portait aucune génération de plan. Le garde-fou d'oisiveté d'`auto_verify` (`verification.is_some() && !verification_stale`) refusait alors de redemander : « Placement vérifié ✓ » figé sur une grille que personne n'a jugée.

Enfin, une friction de lecture : « combinaison automatique — sans conflit ✓ » ne parle que de chevauchements horaires (`core::weekly::schedule_report` ne reçoit ni préalable, ni session antérieure, ni plafond), et se lisait à une ligne de « Placement vérifié ✓ ».

## Décision

**Une épingle est vérifiée comme le reste.** Elle reste permise — l'étudiant peut savoir ce que le répertoire ignore, et le geste est à une « Annuler » près (AIR ACT-2 ; une boîte de confirmation serait du théâtre) — mais elle n'est jamais silencieuse.

- **`solve::pin_warning` est le juge unique de la session**, pur et testé : il réutilise `acquired_before` (le seul contrôle conscient de l'ordre des sessions) et `core::unmet_prerequisites`. Le calcul des causes est extrait dans `prerequisite_causes`, partagé avec `pinned_refusal_causes` : une seule écriture de la règle.
- **`components::panel::place_course` le porte** — la seule porte par laquelle passe tout épinglage : bande de puces, glisser-déposer du ruban, glisser-déposer de la grille. Rien n'est jugé dans `components/` ; la décision est appelée, pas prise.
- **`validate_new_code` ne se prononce plus sur les préalables dès qu'une session est nommée.** Il n'a jamais su ordonner les sessions ; c'est ce qui rendait son verdict faux. Sans session — la puce « automatique », la saisie manuelle — il garde le sien, honnête là où aucun ordre n'existe encore. Les deux notes (été fermé, préalables) étant désormais exclusives, leur fusion disparaît au profit d'un `match session`.
- **Le refus d'une proposition porte sa raison** : `proposal_kept_note` appelle `left_out_line` pour chaque siège réclamé. AIR ALR-1 — une alerte sans geste possible est une entrée de journal.
- **Un verdict ne s'installe que s'il juge la grille affichée** : `SolverState` compte les changements de plan (`plan_generation`), `Running` enregistre ce compte à l'envoi, et `solve::verdict_settles` décide. Une réponse dépassée est gardée (la vider déplacerait tout le panneau, LAT-7) mais reste marquée périmée, ce qui relance `auto_verify` au lieu de le bloquer. AIR TRU-3, appliqué à un verdict au lieu d'une jauge.
- **Le ✓ de l'horaire nomme sa portée** : « sans conflit **d'horaire** ✓ ». Un verdict d'horaire ne doit pas pouvoir se lire comme un verdict de placement (AIR TRU-1).

La fixture `organigrammes/strict-prereq-same-session.json` gèle le phénomène côté noyau — la première à porter à la fois `pinned` et des `prerequisites` non nuls. Deux cours offerts aux deux saisons, tenant ensemble sous le plafond, sans chevauchement hebdomadaire, tous deux épinglés en session 1 : **seule** la précédence peut refuser, et `place.py` en dérive zéro solution, recherche épuisée.

## Alternatives rejetées

- **Corriger le noyau.** La reproduction native montre qu'il refuse déjà, deux fois. Toute correction là-bas aurait été un déplacement du défaut.
- **Interdire l'épingle, ou la faire confirmer.** AIR ACT-2/ACT-3 : sous pression on clique à travers les confirmations. L'acte est déjà annulable ; ce qui manquait était le dire, pas l'empêcher. Et l'étudiant garde le droit de savoir mieux (dérogation, équivalence non encodée).
- **Faire aussi juger `take_verdict` par le portail d'admission complet.** Il refuse un cours « déjà placé en A3 » — c'est-à-dire exactement ce qu'un déplacement fait. Sa sortie anticipée est porteuse pour l'admission ; ce qu'elle ne devait pas court-circuiter, ce sont les préalables, désormais ailleurs.
- **Garder l'avertissement de préalable dans `validate_new_code` et le doubler.** Deux voix pour un même fait, dont l'une fausse, apprennent à ne plus lire les messages.
- **Reclasser les groupes d'un `OU` dont une seule branche siège la même session.** `MAT-1910` exige « MAT-1900 OU MAT-1920* » : aucune des deux issues n'est acquise avant la session, et c'est un manque avant qu'il faut dire, en nommant les deux issues — l'étudiante peut agir sur l'une ou sur l'autre. La partition existante (`préalable suivi la même session` quand *toutes* les issues du groupe siègent là) reste inchangée ; elle sert le préalable strict et unique, qui est le cas courant.
- **Comparer une empreinte de requête plutôt qu'un compteur** pour la fraîcheur du verdict : il faudrait resérialiser la requête à chaque réponse, et `Running` cesserait d'être `Copy`.
