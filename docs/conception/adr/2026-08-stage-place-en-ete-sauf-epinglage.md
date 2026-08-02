# Les stages se placent en été, sauf épinglage — les étés fermés par défaut

**Date :** 2026-08-02
**Statut :** accepté (décision Antoine).

## Contexte

Le solveur B ne connaissait ni les stages ni les sessions d'été : `alternating_sessions` ne générait jamais d'été, et rien ne distinguait un stage d'un cours régulier.
Dans les faits, un stage de génie se fait à l'été entre deux années d'études ; un cours régulier ne se prend l'été que sur décision explicite de l'étudiant.

## Décision

- `PlacementRequest` gagne deux champs :
  - `stages: &BTreeSet<String>` — codes dont le domaine est restreint aux sessions d'été de l'horizon ;
  - `open_summers: &BTreeSet<usize>` — indices 1-based (la convention de `pinned`) des étés ouverts aux cours réguliers. **Défaut : vide** — un été n'accepte que des stages et des cours épinglés.
- **L'épinglage lève les deux règles**, dans les deux sens : un stage épinglé peut aller en automne/hiver, un cours régulier épinglé peut entrer dans un été fermé. Épingler est un acte aussi explicite qu'ouvrir l'été ; l'intersection épingle×offre reste inchangée (épingler vers une saison qui n'offre pas le cours reste `EmptyDomain`).
- La restriction vit dans la **construction du domaine** (`summer_admits`, filtré dans `value_ordered_domain`), pas dans la recherche : le moteur ne change pas.
- Validation typée : `StageWithoutCourse` (un code de `stages` sans `Course` — un stage aussi `passed` est permis, il ne devient jamais candidat), `OpenSummerOutOfRange`, `OpenSummerNotSummer` (l'indice doit pointer une session `summer`).
- Nouveau `BlockedReason::StageWithoutSummer` : un stage non épinglé au domaine vide nomme la restriction été comme coupable — l'épingler est la sortie actionnable ; tout autre domaine vide reste `EmptyDomain`.
- Les fixtures `organigrammes/` gagnent deux clés optionnelles `stages` et `open_summers` ; la référence Python (`place.py`) reflète `summer_admits` et les validations.

## Alternatives rejetées

- **Détecter les stages par leurs données** (crédits Range, absence d'horaire) : GEX-1580 a 9 crédits fixes et un horaire publié — aucun signal fiable ; l'appelant (l'intake, via la règle « Stages ») sait, le solveur reçoit.
- **Un booléen « été ouvert » global** : les étudiants ouvrent un été précis (celui d'un échange, d'un retard à rattraper), pas tous ; les indices portent tous les cas.
- **Interdire l'épinglage d'un cours régulier dans un été fermé** : asymétrique avec le stage épinglé hors été, et l'épingle est déjà le canal du « je sais ce que je fais ».
- **Restreindre par saison dans `Course`** : la règle est un choix de l'étudiant sur *cette* requête, pas une propriété du cours.
