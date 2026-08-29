# La puce déjà choisie parle au lieu de refuser en silence

Date : 2026-08-29

## Contexte

Un rapport de persona du 2026-08-29 signalait que « automatique » et « créditer » ne faisaient rien, silencieusement, à côté d'un cours candidat d'une règle de concentration.

La reproduction exacte a été rejouée dans un navigateur piloté (B-GCI A26, concentration « Cheminement sans concentration », Règle 1 à 0/15 cr, FOR-2020), pour les sept états du menu « Rattacher FOR-2020 à une règle » et les quatre concentrations du programme : **le bogue ne se reproduit pas**.
« Automatique » fait passer l'en-tête de 97/120 à 100/120 et écrit « FOR-2020 laissé au solveur » dans l'historique ; « créditer » écrit « FOR-2020 crédité (entente) ».
Un balayage exhaustif des huit sections du B-GCI (les quatre premières rangées de chacune, les deux boutons) n'a trouvé **qu'une seule** famille de clic muet, et toujours la même : la puce dont `aria-pressed` valait déjà `true`.

La garde en cause est dans `CourseChoice` (`crates/ui/src/components/panel.rs`) :

```rust
let auto = choice == panel::Choice::Auto;
// …
onclick: move |_| {
    if auto {
        return;   // et, pour une puce de session : if here { return; }
    }
```

Elle est légitime — reprendre un cours déjà pris n'a rien à faire — mais elle se déclenche sur **tout obligatoire** (`panel::choice(…)` répond `Choice::Auto` dès que `mandatory`), et un obligatoire n'a pas de « ✕ » (ADR `2026-08-choix-automatique-ou-session-gelee`).
Sa puce « automatique » est donc un bouton qui, pour la vie du cheminement, ne fera jamais rien : ECN-4901, GCI-1000, GCI-1001, GCI-1003, GCI-2580 se comportent tous ainsi.
Le seul signal était le remplissage inversé de `.panel-chip--chosen`, dont le `:hover` promettait par-dessus le marché une action (bordure et fond `--accent`).

## Décision

Une puce qui porte déjà le choix est un **marqueur d'état, pas une action**, et elle le dit :

- `aria-disabled="true"` s'ajoute à `aria-pressed` — jamais `disabled`, qui la sortirait de l'ordre de tabulation et, selon la plateforme, supprimerait l'infobulle : l'explication disparaîtrait avec le clic.
- Le `title` passe de la promesse (« Prendre FOR-2020 et le geler en A1-A26 ») à l'état plus les issues, calculé par `present::chosen_chip_title` : « FOR-2020 est déjà gelé en A1-A26 — choisissez une autre session pour le déplacer, « automatique » pour rendre sa session au solveur, « ✕ » pour le retirer. »
  Un cours imposé n'a pas de « ✕ » : son titre ne l'envoie pas vers un contrôle que la rangée lui refuse, il dit « le programme l'impose, rien ne peut le retirer » (ERR-1 : dire quoi faire, pas seulement que rien ne s'est passé).
- Le `:hover` de `.panel-chip--chosen` cesse de changer de couleur et le curseur redevient `default`.

La garde elle-même ne bouge pas : le comportement était correct, c'est son silence qui ne l'était pas.

## Alternatives rejetées

- **Un toast d'erreur à chaque clic sur une puce déjà retenue** : le geste est fréquent et sans conséquence ; une file de bannières pour « rien à faire » viole ALR (actionnable, limité en débit) et déplace l'explication loin du contrôle.
- **`disabled` sur le bouton** : la façon la plus courte de rendre le refus visible, mais elle emporte l'infobulle qui explique le refus et retire la puce du clavier — on remplacerait un clic muet par un contrôle invisible au clavier.
- **Retirer la puce « automatique » d'un obligatoire** : elle porte l'information « ce cours est pris, le solveur choisit sa session », et elle redevient cliquable dès que l'étudiant gèle le cours dans une session. La supprimer ferait disparaître la seule façon de dégeler.
- **Faire de la puce un interrupteur qui retire le cours** : « automatique » et « ✕ » auraient le même effet sur un cours au choix, et un obligatoire deviendrait retirable — ce que l'ADR `2026-08-choix-automatique-ou-session-gelee` refuse.
