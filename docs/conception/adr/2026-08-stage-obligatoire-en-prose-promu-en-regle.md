# Le stage obligatoire en prose promu en règle « Stages »

Date : 2026-08-02

## Contexte

Les cinq bacs de génie énoncent leur stage de diplomation dans la prose d'un bloc : « En plus des cours obligatoires du programme, l'étudiant doit réussir le stage de formation pratique GEX-1580 pour obtenir son diplôme. […] Les crédits de ces stages sont en sus des crédits exigés du programme. » (génie civil écrit « vous devez réussir … pour diplômer »).
Cette prose était capturée en note programme (ADR `2026-07-notes-en-prose-conservees`) — affichée, mais invisible au vérificateur de règles, alors qu'elle porte une exigence de diplôme.
D'autres programmes mettent leurs stages directement en règles ; une exigence de stage doit être comptable partout.

## Décision

- Après `extract_language_requirement`, le parseur promeut chaque note programme reconnue comme prose de stage en une règle ordinaire ajoutée **à la suite** des règles scrapées : titre « Stages », contrainte `{"type": "course", "min": 1, "max": 8}` (au moins le stage obligatoire ; jusqu'aux huit stages que la direction autorise — aucune borne n'est lisible sur la page), et la liste de tous les sigles de la note dans l'ordre, l'obligatoire en tête.
  La note entière survit comme note de la règle : la distinction obligatoire/optionnels et le pointeur d'inscription ne sont lisibles nulle part ailleurs.
- Prédicat volontairement étroit, en verrou avec les cinq pages : « réussir le stage » **et** « diplôm » **et** un sigle ; portée `program.notes` seulement (les cinq occurrences y vivent).
  Une prose qui n'y répond pas reste une note — rien n'est inventé.
- Nouveau champ `Rule.credits_in_addition` (sérialisé seulement quand vrai) : « en sus des crédits exigés » — la règle doit être satisfaite mais ses crédits ne comptent pas dans `credits_required`.
  Le drapeau suit la **source** : seule une note « en sus » le pose ; une règle de stages native resterait à false.
- Ondulation : les 5 fixtures programmes régénérées (la note disparaît, la règle « Stages » apparaît en fin de `rules`), les 14 fixtures rules resynchronisées et leurs attendus re-dérivés par la référence (chaque rapport de scope programme gagne une entrée « Stages », incomplète tant qu'aucun stage n'est sélectionné).

## Alternatives rejetées

- **Rester en note** : le vérificateur ne peut pas compter une exigence qu'il ne voit pas.
- **Ne lister que le stage obligatoire** : perdrait les stages optionnels comme candidats ; la liste complète ordonnée garde l'obligatoire en tête sans rien perdre.
- **Un marquage « en sus » uniforme sur toute règle de stages** : contredirait le baccalauréat en éducation au préscolaire et en enseignement au primaire (BEPEP), où le bloc des stages compte dans les 123 crédits du programme (93 + 30) ; le drapeau suit la source.
- **Capturer du même coup les règles de stages natives du BEPEP** — « Règle 1 – Un stage parmi : », « Règle 2 – 3 stages parmi : », chaque option « Stage N : » groupant un ou deux cours (partie 1 + partie 2, à prendre ensemble) : demanderait une grammaire « N stage(s) parmi », une variante `RuleCourses::Groups {label, courses}` et l'appariement libellés/cartes en ordre de document dans `parse_accordion`.
  **Remis à plus tard** ; consigné ici pour la reprise, avec la page témoin : <https://www.ulaval.ca/etudes/programmes/baccalaureat-en-education-au-prescolaire-et-en-enseignement-au-primaire#section-structure>.
