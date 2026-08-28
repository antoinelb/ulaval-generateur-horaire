# Un toast par sujet, et un rejet qui se souvient

Date : 2026-08-27

Amende `2026-08-alertes-en-toasts-flottants`.

## Contexte

Rapports étudiante 2026-08-27 (G3 et F1) : le bandeau « Le cheminement présume ces acquis… » s'affichait en double, et rejeter un message le ramenait à la réponse suivante du solveur.

Trois faits se composaient.
`apply_proposal` republie **toutes** ses notes à chaque réponse du solveur, avec la cause `Document`.
La déduplication ne portait que sur le corps **exact** : une liste d'acquis qui gagne un sigle est un autre corps, donc un second bandeau à côté du premier.
Et les alertes `Document` ne périment qu'à la bascule de document — changer de concentration n'en est pas une.
Le rejet, lui, était un simple `retain` sans mémoire : la réponse suivante repoussait le même message.

## Décision

Un **sujet** (`AlertTopic`) est ajouté à côté de la **cause**, et la pile devient un type pur.

- Nouveau module `crates/ui/src/alerts.rs` (hors `components/`, donc couvert à 100 %) : `Alert`, `AlertBody`, `AlertCause` y déménagent, plus `AlertTopic` et `AlertStack`.
- **La cause décide de la péremption, le sujet du remplacement et du rejet.** Étendre `AlertCause` aurait mélangé les deux questions : deux notes peuvent partager une cause (`Document`) sans parler du même sujet, et deux notes du même sujet peuvent avoir des causes différentes.
- `push_topic(body, cause, topic)` : retire toute alerte du même sujet **et** toute alerte du même corps, puis n'affiche rien si le sujet a été rejeté avec exactement ce corps ; sinon pousse et efface la mémoire du sujet.
- `dismiss(key)` — le geste de l'étudiant, ✕ ou clic sur le message — mémorise sujet → corps.
- `retire(pred)` — péremption par cause, minuterie des ✓, « Annuler » du toast programme — ne mémorise **jamais** : l'étudiant n'a rien dit.
- `purge_document()` vide la mémoire avec les alertes du document.
- Les sept catégories de `apply_proposal` sont étiquetées : `Completion`, `EmptyGrid`, `LeftOut(code)`, `SetAside(code)`, `SummersForced`, `Assumed`, `Injected` — plus `ProposalKept` (`2026-08-proposition-refusee-si-elle-desassoit`).

ALR-3 (jamais deux fois le même message) et ALR-4 (persiste jusqu'au rejet) tiennent inchangés.

## Alternatives rejetées

- Une mémoire de rejet globale par corps : un message légitimement réémis après un vrai changement de plan resterait muet à jamais.
- Auto-effacer les ⚠ comme les ✓ : ALR-4 l'interdit, et c'est exactement ce que l'ADR amendé a tranché.
- Périmer les alertes `Document` au changement de concentration : traiterait le symptôme d'un sujet, pas la republication de tous les sujets à chaque réponse.

## Conséquences

- `AlertBody::LocalProgramRemoved` porte un `Box<LocalProgram>` : lintée nativement, la variante rendait chaque alerte de la pile aussi grosse que la plus rare.
