# Le sélecteur « Début » descend huit ans sous l'horloge

Date : 2026-08-30

## Contexte

L'ADR `2026-08-retrait-de-la-notion-de-cours-reussi` a supprimé « marquer réussi » en s'appuyant sur un chemin de rechange, énoncé dans sa dernière ligne : « Un étudiant en cours de bac exprime ses acquis en plaçant ses cours faits dans les sessions passées de l'horizon — l'horizon couvre toujours le début du bac. »

Ce chemin n'existait pas.
Le rapport du directeur du baccalauréat en génie civil (`docs/ux/rapport-directeur-gci-2026-08-30.md`, gravité bloquante) le constate : il n'a trouvé **aucun** moyen de dire « cet étudiant a coulé GCI-1001 » sans un vrai relevé Capsule — le scénario central de son mandat, celui qu'il pose le plus souvent en comité de programme.
Il a vérifié le sélecteur : sous une horloge A26, ses seules options sont A26 … H31, aucun semestre passé.

Le verrou est une seule ligne, `crates/ui/src/state.rs` :

```rust
(start_year.min(today_year), start_year.max(today_year + 5))
```

La borne basse `min(start_year, today_year)` n'est motivée nulle part.
L'ADR `2026-08-debut-ancre-sur-lhorloge` ne justifie que la borne **haute** (« cinq ans au-delà de l'horloge, il y a de la place pour planifier ») et déclare déjà souverain « le réglage manuel du sélecteur ».
`set_start` n'a aucune borne temporelle : il accepte déjà n'importe quel semestre, et `persist::restore_plan` laisse déjà passer un début passé sauvegardé.
Le sélecteur était le seul obstacle, et il l'était par accident.

Sans début dans le passé, aucune session de l'horizon n'est dans le passé ; sans session passée, il n'y a nulle part où poser un cours fait ; sans cours fait, l'unique mécanisme que l'interface offre depuis le retrait des ✓/↩ est hors de portée.

## Décision

La borne basse descend à huit ans sous l'horloge, la borne haute ne bouge pas :

```rust
(
    start_year.min(today_year.saturating_sub(PAST_START_YEARS)),
    start_year.max(today_year + 5),
)
```

Huit ans : le double de la durée nominale d'un bac de génie, soit la durée la plus longue qu'un parcours à temps partiel traîne réellement avant de cesser d'être planifiable.
La fenêtre continue par ailleurs de s'élargir au début réel du plan — un relevé Capsule peut l'ancrer plus loin encore, et une liste d'options sans cette année-là laisserait le `<select>` afficher en silence la mauvaise session.

**`study_sessions` ne bouge pas.**
Reculer le Début décale l'horizon vers le passé ; l'étudiant augmente « Sessions » lui-même s'il veut retrouver la fin de son parcours.
Un rallongement automatique inventerait des sessions que personne n'a demandées et déplacerait sous le curseur la grille entière au moment même où le directeur change un réglage.

Rien d'autre ne change.
`set_start` continue d'évincer **et de nommer** les sièges qu'une nouvelle saison n'offre plus (TRU-3), le gel continue de voyager par semestre, le ruban continue de griser toute session strictement antérieure à l'horloge (`present::ribbon_model`, champ `passed`), et le solveur traite ces sessions comme n'importe quelles autres : un cours qui y siège satisfait les préalables des sessions suivantes.

## Ceci ne contredit pas `2026-08-debut-ancre-sur-lhorloge`

Le mal que cet ADR-là visait était une **valeur par défaut dans le passé** : un A26 écrit en dur dans `Plan::default()`, servi tel quel à une étudiante planifiant en 2028.
Le correctif était `floor_start`, qui **relève** — et qui reste raise-only : `fresh_plan`, `restore_plan` (sur le plan exactement égal à l'usine) et `reset_document` plancherent toujours un document neuf sur l'horloge.

Offrir un passé n'est pas y défaillir.
La valeur par défaut reste l'horloge ; ce qui change est la liste des choix qu'un étudiant, ou un directeur, peut prendre **explicitement**.
Un début passé explicite était déjà souverain — relevé Capsule, lien partagé, réglage manuel — et le test `persist::a_stored_past_start_survives_restore` le prouvait avant ce changement.
Le sélecteur cesse simplement d'être le seul chemin explicite fermé.

## Alternatives rejetées

- **Réintroduire « marquer réussi »** (✓/↩ par cours, `Plan.passed`) : réverserait `2026-08-retrait-de-la-notion-de-cours-reussi` pour une information que le placement porte déjà, et rouvrirait le double marquage qu'Antoine avait fait retirer. La bonne réponse est de rendre atteignable le mécanisme que cet ADR-là avait promis, pas d'en ajouter un second.
- **Un formulaire de relevé fictif** à côté du collage Capsule : une deuxième porte d'entrée à écrire, à valider et à tester, pour un cas que trois clics dans le sélecteur couvrent.
- **Rallonger `study_sessions` automatiquement au recul du Début** : voir plus haut — décision explicite d'Antoine.
- **Descendre la borne basse à l'infini** (aucune borne) : une liste de plusieurs centaines d'options, sans valeur d'usage — un bac commencé avant 2018 ne se planifie plus, il se termine.
- **Une case « afficher les sessions passées »** qui déverrouillerait la liste : un réglage de plus à comprendre et à persister, devant une liste qui n'a jamais eu besoin d'être verrouillée.

## Conséquences

- Le sélecteur passe d'environ 12 options à environ 28 — un `<select>` natif, défilable et complet au clavier ; sa hauteur à l'écran ne change pas.
- Le scénario du directeur est atteignable sans relevé Capsule : reculer « Début » jusqu'à la session d'admission réelle de l'étudiant, poser les cours faits dans les sessions grisées, laisser hors placement (ou déplacer plus loin) celui qui a été coulé, puis lire la replanification.
- Un recul d'un nombre impair de créneaux change les saisons : le bilan de `set_start` nomme alors les cours retirés, comme il le faisait déjà pour une avance.
