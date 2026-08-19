# Ententes cours→règle et case « scolarité préparatoire faite »

Date : 2026-08-13

Révisé par `2026-08-entente-vers-une-regle-any.md` : une règle « any » est désormais une cible d'entente (elle n'« acceptait » rien — core ne la comptait jamais).

## Contexte

Deux besoins des notes d'essai d'Antoine (2026-08-13) :

- Un étudiant peut avoir une **entente avec la direction** pour qu'un cours — manuel ou du catalogue — compte dans une règle où il n'est pas normalement admis. L'interface n'offrait rien : le cours manuel n'apparaissait que dans la session, jamais dans les règles du programme.
- La règle « Scolarité préparatoire » doit porter une case « déjà faite », **cochée par défaut** (note 11).

## Décisions

- **L'entente est une donnée, pas une logique** : `Plan.rule_grants` (code → clé de section « p/Règle 2 », « c/… », « f/… ») et la fonction pure `panel::granted_program` qui clone le `Program` et ajoute chaque code accordé à la liste de sa règle **avant** tout appel à `core::coverage_report` ou tout envoi au worker (`panel::effective_program`). La couverture reste comptée par core ; l'UI ne fait que décrire l'accord.
- Une règle **« negotiated »** (*convenus avec la direction* — mot-clé reconnu par l'ADR `2026-07-regles-negociees-reconnues`) devient la **liste de ses ententes** : c'est exactement le cas que ce mot-clé attendait, et la règle passe de « rapportée » à « comptée ».
- Une entente inapplicable (règle introuvable après changement de programme, règle « any » qui accepte déjà tout, forme sans liste) est **nommée dans le panneau**, jamais perdue.
- **Une entente déplace, elle ne duplique pas** (révision post-essai, rapport `2026-08-13b`) : le code accordé est retiré de la liste de toute autre règle avant d'être ajouté à sa cible — sinon un même cours créditait deux règles à la fois. La règle « Scolarité préparatoire » n'est **jamais** une cible d'entente (y rattacher un cours le rendrait « acquis » d'office par la case cochée).
- Chaque rangée de cours porte un petit select « entente… » (rattacher / retirer, undoable) ; le formulaire de cours manuel offre le même choix à la création ; une rangée accordée est marquée « · entente ».
- **Case préparatoire** : `Plan.preparatory_done` (défaut `true`), rendue sur la section « Scolarité préparatoire ». Cochée : les 0xxx de la règle voyagent en `PlaceQuery.passed` (jamais placés, ADR `2026-08-retrait-de-la-notion-de-cours-reussi`) et rejoignent la sélection de couverture (badge ✓). Décochée : ils redeviennent du travail ordinaire à placer.

## Alternatives rejetées

- Compter l'entente côté UI (ajuster les badges après coup) : duplique la logique de comptage de core — violation directe de « aucune règle métier dans la vue ».
- Étendre `core::Rule` d'un champ d'ententes : core n'a pas à connaître un accord local à un étudiant ; la chirurgie de données en amont suffit et reste testée.
- Offrir l'entente sur les « Obligatoires » : là, l'accord réel est une équivalence de cours, pas une appartenance de règle — hors portée.
