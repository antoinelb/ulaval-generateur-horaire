# Choisir un cours d'abord, geler sa session ensuite

Date : 2026-08-17

## Contexte

Le panneau n'offrait qu'une porte d'ajout : le bouton « + » d'une rangée écrivait le cours dans `plan.manual[view.session]`, donc l'épinglait à la **session affichée** sans jamais le dire.
Le geste intuitif — « je veux ce cours, débrouille-toi pour le placer » — n'existait nulle part, alors que le modèle le portait déjà : `Plan.electives` = choisi sans session, `Plan.pinned_sessions` = gelé quelque part.
L'état `RowState::Chosen` (« choisi - à placer par le solveur ») attendait depuis le jalon 9 un producteur que personne n'avait écrit ; seule l'injection automatique du solveur y menait.

Les sessions possibles se demandaient à part, par un bouton « où le placer ? » qui lançait une sonde `core::admissible_sessions` — un solve complet par cours, à la demande.

## Décisions

- **Une bande de choix par rangée** remplace le « + », le bouton « où le placer ? » et les deux « ✕ » : `[automatique] [A1-A26] [H2-H27] …`.
  « Automatique » prend le cours et laisse le solveur choisir sa session (`electives`, pas d'épingle) ; une puce de session le prend **et le gèle** là (`place_course` : élective + `pinned_sessions` + `displayed_placement`).
  Cliquer une autre puce change le choix — c'est un déplacement, pas un retrait suivi d'un ajout.
- **Le choix retenu est marqué visuellement seulement** (puce pleine, `aria-pressed`), et la rangée choisie prend une bordure plus foncée.
  Exception assumée à INP-3 : le sous-titre dit déjà « choisi - à placer par le solveur » ou « placé en H2-H27 », donc le texte porte l'information partout où elle change une décision.
- **Un cours obligatoire n'a pas de « ✕ »** et affiche « automatique » d'emblée : le solveur le place déjà tout seul (le `Program` part entier dans `PlaceQuery`), l'interface cessait donc de dire la vérité en offrant de l'« ajouter ».
  Ses puces de session restent : le geler quelque part est un choix légitime.
  Obligatoire se lit sur le programme *et* sur la concentration et le profil retenus, pas sur la section qui affiche la rangée — un obligatoire listé sous une règle est marqué là aussi.
- **La liste des sessions est un filtre de saison local** (`course.seasons` croisé avec l'horizon), pas une sonde du solveur.
  Une session barrée par les préalables reste cliquable ; `validate_new_code` avertit alors, comme il l'a toujours fait.
- **La porte de validation devient `session: Option<usize>`** : sans session, seuls les contrôles liés à la session s'effacent (déjà dans cette session, saison, été fermé).
  Le reste — code introuvable, préparatoire cochée, crédité par entente, déjà placé ailleurs, avertissement de préalables — vaut pour les deux gestes.
  Elle ne s'exécute qu'à la **première** prise : changer de puce est un déplacement déjà accepté une fois.
- **Le retrait passe par `state::purge_codes`**, qui nettoie déjà élective, épingle, placement, ajout manuel et sections forcées. `remove_course` disparaît, et `place_course` s'en sert pour déplacer.
- **`plan.manual` n'a plus de producteur** : le champ « Ajouter par code… » et le formulaire de cours manuel choisissent eux aussi sans session. Le champ et ses lecteurs restent, pour les plans enregistrés et parce qu'une entrée `manual` se lit encore comme un gel.

## Alternatives rejetées

- **Sonder `admissible_sessions` pour chaque rangée visible** : exact (préalables compris), mais 5 à 20 solves à chaque ouverture de section. Le filtre saison donne la même liste dans presque tous les cas, en mémoire.
- **Filtre saison affiché tout de suite, puis sonde en arrière-plan qui grise les sessions inadmissibles** : exact et instantané, mais deux vérités successives à l'écran et le plus de code des trois.
- **Garder le « + » et ajouter une puce « automatique » à côté** : deux gestes pour la même chose, dont un qui ment sur sa cible.
- **Un « ✕ » sur les obligatoires qui ne retirerait que l'épingle** : une action dont le sens diffère de la même icône partout ailleurs. La puce « automatique » dégèle déjà, et c'est le même geste que sur un cours au choix.
