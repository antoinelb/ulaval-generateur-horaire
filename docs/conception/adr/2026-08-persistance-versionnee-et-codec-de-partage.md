# Persistance localStorage versionnée et codec d'URL de partage

## Contexte

L'état de l'utilisateur vit côté client (invariant : aucun backend) et doit survivre au rechargement (AIR ACT-7 : l'entrée de l'opérateur n'est jamais perdue), tout en restant lisible par les versions futures de l'app sans jamais rien perdre silencieusement.
Le partage d'un horaire passe par l'URL (un horaire choisi n'est qu'un ensemble de sections, `docs/project_plan.md` § Transversal).

## Décision

- **Trois clés localStorage** : `gh.v1.plan` (le document annulable — programme, réussites, épinglages, placement affiché, sections choisies, ajouts manuels), `gh.v1.view` (navigation : session affichée, onglet, filtres, densité — restaurée mais jamais annulable), `gh.v1.log` (anneau borné à 200 entrées : corrections, erreurs, latences — OBS-2/OBS-6).
- **Enveloppe versionnée** `{version, state}` ; la restauration est tolérante et *bruyante* : champ absent → défaut sans cérémonie ; champ inconnu → nommé dans une note **et** copie de sauvegarde remise à l'appelant ; version plus récente, JSON illisible, contenu incompatible → reprise à neuf + note + copie. L'inverse exact des entrées solveur (`deny_unknown_fields`) : typer strictement, restaurer généreusement.
- **Codec de partage** `?h=<session>.<CODE[:NRC+NRC…]>,…` (ex. `a2027.GCI-1007,GEX-1000:12345+12346`) : clé de session du nommage `schedule_intake`, NRC triés (une option n'a pas d'identité au-delà de son ensemble de NRC, ADR `2026-07-contrat-horaire-hebdomadaire-vers-ui`), un cours sans section = code nu. Chaque caractère est URL-sûr sans échappement — le lien reste lisible par un humain. Toute malformation est une erreur typée, jamais un import partiel.
- **L'arithmétique calendaire vit dans `ui::state`** (`session_semesters` : l'hiver appartient à l'année civile suivant son automne), pur et testé — pas dans `core`, qui ne connaît que des saisons ordonnées (« l'horizon est décrit, jamais listé »).

## Alternatives rejetées

- **Une seule clé fourre-tout** : l'annulable et le navigationnel n'ont pas le même cycle de vie (undo ne doit pas téléporter l'écran).
- **Migrations muettes** : réparer sans le dire est la définition de la perte silencieuse.
- **Encodage binaire/base64 de l'URL** : plus court mais opaque ; le format lisible se déboque à l'œil et se partage en confiance.
