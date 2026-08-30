# Un lien de partage rouvre toujours un organigramme gelé

Date : 2026-08-30

S'appuie sur `2026-08-sessions-gelees-generalisent-les-completees` et `2026-08-bouton-tout-geler-dans-la-barre-du-haut`; répond au `ponytail:` laissé ouvert par `2026-08-sessions-completees-fermees-au-solveur`.

## Contexte

Retour d'un lecteur du prototype, transmis par Antoine le 2026-08-30 :

> Je pense qu'il faudrait cependant « brider » la modification automatique de cheminements pour que le même url donne toujours le même cheminement, quitte à voir des erreurs apparaître dans celui-ci si des modifications aux préalables et horaires des cours sont faites dans l'outil en cours de route. Dans mon esprit, on devrait être en mode « manuel » par défaut et il faudrait cliquer sur un bouton pour que le logiciel essaie de modifier l'horaire automatiquement pour satisfaire toutes les contraintes.

Le grief est fondé, et trois faits déjà écrits se combinent pour le produire :

1. `share_into` (`persist.rs`) pose `frozen: BTreeSet::new()` — le codec ne transporte pas le gel. Un organigramme entièrement gelé chez l'expéditeur arrivait **entièrement dégelé** chez le destinataire.
2. `displayed_placement` voyage bien dans le lien, mais `solve.rs` ne l'envoie que comme **graine**, pas comme contrainte. C'est `frozen` qui la durcit : sans lui, `wasm::organigramme::with_request` n'en épingle aucune entrée.
3. `auto_propose` tourne 500 ms après l'import (ADR `2026-08-organigramme-en-continu-sans-bouton`) et réécrit `displayed_placement` par une écriture directe, hors pile d'annulation.

Un même lien pouvait donc rouvrir un cheminement replacé par le solveur, différent de celui que l'expéditeur avait sous les yeux — et ce silencieusement, puisque l'adoption d'une proposition n'est pas un acte annulable. C'est exactement ce que le commentaire `ponytail:` de `persist.rs` annonçait (« add the field to the share state if that ever misleads »).

Le « mode manuel » réclamé existe depuis le 2026-08-30 : le gel de session, sa case dans chaque carte, et le bouton « Tout geler » de la barre du haut. Il manquait de l'appliquer d'office quand le document n'est pas celui qu'on a construit soi-même.

## Décision

**Un organigramme importé par lien arrive avec tout son horizon gelé**, écrit dans la même fermeture d'`edit_plan` que le remplacement du plan.

- Le gel est **forfaitaire**, calculé à l'arrivée par `present::whole_horizon` — l'ensemble complet des sessions de l'horizon déplié, étés compris. Le codec n'a donc rien de neuf à transporter : `Share` ne bouge pas, ni la constante du test `the_frozen_string_still_encodes_byte_for_byte`.
- `whole_horizon` est **extraite de `freeze_all`**, dont elle était le calcul interne, et l'import l'appelle elle et non `freeze_all` : cette dernière est une **bascule**, qui rendrait un ensemble vide le jour où un plan importé arriverait déjà gelé. Une fonction qui décide « tout l'horizon » ne doit pas pouvoir répondre « rien ».
- Ce gel suffit à figer le document au cours près : `with_request` épingle chaque entrée de la graine assise dans une session gelée (ADR `2026-08-sessions-gelees-generalisent-les-completees`), et la graine *est* le `displayed_placement` du lien.
- **Dans le même acte**, jamais en deux écritures : « Annuler » doit rendre le document de l'étudiante, jamais un état intermédiaire dégelé qui n'a existé pour personne (ACT-2).
- **L'avis le dit** — l'interface ne change pas un état en silence (TRU-1), et il nomme la sortie (ALR-1) : « Organigramme partagé importé, toutes ses sessions gelées : il rouvre tel que l'expéditeur l'a envoyé. « Tout dégeler » rend la main au solveur, « Annuler » restaure le vôtre. » Le libellé cité est celui que `freeze_all` rend sur un plan entièrement gelé, mot pour mot.

Un cours que le destinataire ne peut pas asseoir — millésime de programme différent, cours absent de son catalogue — n'est pas replacé en douce : il ressort « mis de côté » par le repli `allow_unplaced` (ADR `2026-08-placement-au-mieux-en-repli`). C'est l'erreur visible que le retour demande explicitement, préférée à un cheminement discrètement réécrit.

## Alternatives rejetées

- **Porter `frozen` dans le codec de partage** : plus de code (un champ de plus dans `Share`, la chaîne du test d'octet à refaire) pour un résultat **plus faible**. Un expéditeur au gel partiel enverrait un lien que le solveur du destinataire peut encore remanier sur le reste — soit le grief, intact. Le lien porte un document, pas un mode d'édition.
- **Un mode « manuel / automatique » global, avec son bouton** : c'est la lettre de la demande, mais un état de plus à porter, à sauvegarder, à afficher et à expliquer, qui recouvrirait le gel sans le remplacer. « Tout geler » fait déjà le travail, se lit sur chaque carte, et n'a pas de second endroit où mentir.
- **Ne geler que les sessions garnies** : une session vide laissée dégelée reste une porte par où le solveur déplace un cours venu d'ailleurs — et les étés sont précisément vides.
- **Suspendre `auto_propose` après un import** : le solveur cesserait alors de *vérifier* le cheminement reçu, or c'est lui qui fait apparaître les erreurs demandées. Geler contraint la recherche sans l'éteindre ; la débrancher rendrait le document muet.
- **Geler aussi à la restauration depuis `localStorage`** : hors demande. Un document qu'on a construit soi-même et qu'on rouvre est le sien; le figer d'office confisquerait le solveur à l'usage courant.

## Conséquence assumée

« Partager » continue d'écrire le lien dans la barre d'adresse (`browser::set_fragment`), et rien ne l'en retire ensuite : c'est le filet de secours quand le navigateur refuse le presse-papiers, et le message d'échec y renvoie explicitement (ERR-1, ADR `2026-08-partager-confirme-ou-dit-son-echec`). Recharger cet onglet ré-importe donc son propre organigramme, désormais gelé, avec l'avis qui va avec.

C'est conforme à l'invariant qu'on vient de poser — une URL ouvre du gelé — c'est annulable, et c'est le prix de garder le filet. Arbitré ainsi avec Antoine le 2026-08-30, après avoir envisagé de ne plus toucher à la barre d'adresse du tout.
