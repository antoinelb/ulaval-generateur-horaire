# L'organigramme se met à jour en continu, sans bouton

Date : 2026-08-19

## Contexte

Demande d'Antoine (2026-08-19) : à chaque ajout, retrait ou déplacement de cours, l'organigramme doit se remettre à jour de lui-même, avec le moins de changements possible ; « Proposer un organigramme » ne doit pas exister.
L'ADR `2026-08-verification-automatique-du-cheminement` avait gardé « Proposer » et « Chercher plus longtemps » en boutons (« des actions qui changent le document, pas des lectures ») — cette décision est **supersédée** ici.

## Décisions

- **Un effet `auto_propose`**, même motif que `auto_verify` (débounce 500 ms, compteur-génération) : dès que le plan se pose avec des cours sans session — ou qu'une vérification a répondu « aucune solution » (`verification.solutions` vide : un geste de l'étudiant a invalidé la grille ; pas `verify_failed`, qui marque une erreur *technique* du worker et refuserait la réparation aussi) — une requête `place` part d'elle-même, au petit budget (200 000 nœuds). La réparation respecte les épinglages (contraintes dures) et réorganise le reste ; un épinglage réellement infaisable donne le remplissage au mieux, ses laissés-pour-compte nommés au verdict, l'épinglage jamais défait.
- **« Le moins de changements possible » est le `seed`** : chaque requête envoie déjà `displayed_placement` comme seed et le cœur trie chaque domaine par distance à lui — proximité sans preuve de minimalité, assumée depuis `2026-07-b-placement-par-satisfaction-fait-main`.
- **Convergence par empreinte** : la requête elle-même (id 0) est enregistrée dans `SolverState.proposed` **à l'envoi** ; la même requête n'est jamais renvoyée. C'est ce qui arrête la boucle quand la réponse ne change rien (`left_out` persistant, réparation impossible) et ce qui rend « Annuler la recherche » effectif jusqu'à la prochaine édition. Un redémarrage *système* du worker (corrections de millésime, cours manuel ajouté) efface l'empreinte : le nouveau catalogue peut répondre autrement.
- **La proposition appliquée est une correction dérivée** : écriture directe du plan, pas d'entrée d'annulation (même raisonnement que `heal_acquired`) — annuler l'acte qui a fait bouger le solveur restaure le plan entier, et le placement se recalcule. L'écriture n'a lieu que si le placement change réellement.
- **`left_out` appartient aux réponses de proposition** : l'effet qui périme le verdict à chaque édition ne l'efface plus — l'auto-application est elle-même une édition et effaçait ce que sa propre réponse venait de rapporter.
- **Les deux boutons disparaissent.** Une recherche tronquée est signalée par l'alerte existante (jamais silencieuse, `2026-07-budget-de-b-en-double-borne`), sans recours manuel — choix d'Antoine.

## Alternatives rejetées

- Tout épingler pour « zéro changement » : dès qu'un ajout force un déplacement, la requête devient insoluble — c'est exactement ce que `verify` fait, et pourquoi elle prouve au lieu de construire.
- Un objectif de minimalité dans le cœur (nombre de cours déplacés) : fausse précision déjà rejetée par `2026-07-b-placement-par-satisfaction-fait-main` ; le seed suffit.
- Auto-relancer au gros budget (1 M de nœuds) quand la recherche rapide tronque : des recherches longues non sollicitées à chaque édition ; Antoine préfère le signalement seul.
- Réutiliser `edit_plan` avec une entrée « Organigramme proposé appliqué » : à fréquence automatique, l'historique (100 entrées) se remplirait de non-actes et chaque annulation surprendrait.

## Conséquences

- Un lien partagé ou un rechargement avec des cours flottants se place tout seul au démarrage.
- `FULL_MAX_NODES`, le champ `SolverState.truncated` et les styles des deux boutons sont retirés.
