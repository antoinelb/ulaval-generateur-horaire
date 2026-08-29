# Un cheminement sans session se charge quand même

Date : 2026-08-29

## Contexte

Une évaluation persona du 2026-08-29 a déposé dans « Charger depuis JSON… » un fichier de la forme

```json
{ "completed": ["GCI-1000", "GCI-1011", "GLG-1000"], "sessions": [] }
```

— « j'ai des cours réussis, je n'ai encore rien planifié ».
Rien ne s'est passé à l'écran; « Annuler » est resté éteint.

`ui::cheminement::apply` lisait la session d'admission ainsi :

```rust
let start = cheminement
    .sessions
    .first()
    .map(|session| session.semester)
    .filter(|semester| semester.season != Season::Summer)
    .ok_or(CheminementError::NoAdmission)?;
```

Un `sessions` vide et un `sessions` ouvrant sur un été tombaient donc sur le même refus, avant même que `completed` soit regardé.
Le refus était typé et affiché — le panneau montre bien le bloc d'erreur — mais il arrive au bas d'un panneau qui défile, si bien qu'à l'usage il se lit comme un silence.
Surtout, c'était le mauvais verdict : un cheminement réduit à `completed` est structurellement valide selon l'ADR `2026-08-un-cheminement-par-fichier`, où le document *est* la grille et rien d'autre.

## Décisions

- **Un `sessions` vide se charge.**
  Les deux cas que le `ok_or` confondait sont séparés : une liste vide ne nomme *aucun calendrier*, une liste ouvrant sur un été nomme un calendrier impossible.
  Seule la seconde reste `NoAdmission`.

- **Le fichier ne décide que de ce dont il parle.**
  `CheminementApplication.timeline: Option<CheminementTimeline>` vaut `None` quand le fichier ne porte aucune session, et `apply_to_plan` garde alors le `start` et l'horizon du plan — au même titre que le plafond de crédits, la concomitance et le programme, que le fichier n'a jamais décidés.
  Inventer un A26 par défaut aurait affirmé un fait que le fichier ne porte pas.
  `start` et `study_sessions` voyagent dans une structure commune plutôt qu'en deux `Option` : un début sans longueur n'est pas un état qu'un fichier peut produire.

- **Le bilan dit ce qui n'a pas bougé** :
  « Aucune session dans le fichier : l'horizon n'a pas bougé, 3 crédités, 0 ignoré. »
  Sans cette phrase, un chargement qui ne touche ni la grille ni l'horizon est indistinguable d'un clic sans effet — exactement ce que la persona a rapporté.
  La porte du catalogue et le bilan des sigles refusés valent inchangés dans ce cas.

- **Les cas voisins restent nommés, aucun n'est muet** :
  `{ "completed": [], "sessions": [] }` — rien à charger — donne `Empty`;
  un document sans champ `sessions` donne `Unreadable` dont le détail nomme le champ manquant (`Cheminement` n'a pas de `serde(default)`, et c'est voulu : `sessions` mal orthographié doit rester une erreur qui nomme le champ, pas un document vide).

## Alternatives rejetées

- **Garder le refus et mieux le rédiger** : le fichier est valide; refuser un document valide pour lui expliquer pourquoi ne mène nulle part.
- **Faire de `sessions` un champ à `serde(default)`**, pour que l'absence vaille le vide : une faute de frappe (`session`) deviendrait alors un document vide refusé par `Empty`, au lieu d'une erreur nommant le champ. Moins précis.
- **Faire démarrer l'organigramme à `Plan::default().start`** quand le fichier ne dit rien : affirme A26 comme si le fichier l'avait écrit, et écrase l'horizon que l'étudiant venait de régler.
