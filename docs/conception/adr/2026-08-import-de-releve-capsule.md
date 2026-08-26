# Import d'un relevé Capsule collé

## Contexte

Le relevé de notes Capsule (la page « Relevé de notes non officiel ») liste les sessions déjà vécues par l'étudiant : cours réussis, cours en cours, crédits reconnus d'un autre établissement.
Le taper à la main dans le panneau est fastidieux et source d'erreurs, alors que l'étudiant peut copier la source de la page (ctrl-u, puis ctrl-a, ctrl-c) et la coller dans l'app — la page rendue seule (ctrl-a/ctrl-c sans ctrl-u) ne porte pas le HTML que le parseur attend.
Le HTML est server-rendu, sans en-tête CORS exploitable et derrière une authentification : aucun fetch automatique n'est possible, seul le collage manuel l'est.

## Décision

### Parseur (`core::parser::transcript`)

- Le HTML collé est lu ligne à ligne (`<tr>`), en pilotant une petite machine à états sur trois sections reconnues par leur bannière (`th.ddtitle`) : « CRÉDITS DE L'UNIVERSITÉ LAVAL » (`TranscriptSection::Laval`), « RECONNAISSANCE DES ACQUIS » (`Recognized`), « CRÉDITS EN COURS » (`InProgress`).
- Chaque session s'ouvre sur un en-tête `span.fieldOrangetextbold` du type « Automne 2024 » ; les lignes de cours qui suivent (sigle, cycle, titre, note le cas échéant, crédits) rejoignent la session active.
- Une ligne reconnue par sa forme mais hors grammaire (en-tête de colonne, ligne de total, séparateur, ligne vide) est ignorée sans commentaire ; toute ligne que la grammaire ne sait pas placer est une anomalie surfacée (`ParseError`), jamais avalée en silence.
- « CRÉDITS EN COURS » n'a pas de colonne note (la session n'est pas terminée) : `TranscriptCourse.grade` y est toujours `None`.

### Application au plan (`core::transcript::apply_transcript`)

- Notes de passage : `D` et mieux, plus `P` (`PASSING_GRADES`) ; un cours Laval hors de cette liste est un échec, ignoré (`IgnoredReason::Failed`).
- Un échec n'est jamais reporté à l'étudiant comme un blocage : il est simplement écarté, le cours restant à planifier normalement.
- Une reprise (échec puis réussite du même sigle) ne compte qu'une fois : les sessions sont parcourues dans leur ordre chronologique et la dernière occurrence d'un sigle l'emporte, donc la réussite efface l'échec antérieur.
- « RECONNAISSANCE DES ACQUIS » ne porte pas de session à l'étudiant : un cours noté `V` rejoint `credited` directement, sans jamais influencer la session de début, l'horizon ou les étés ouverts ; toute autre note y est une anomalie (`UnexpectedGrade`).
- `start` est la plus ancienne session automne/hiver effectivement vécue (ou en cours) à Laval, **parmi celles du programme actuel** ; un été ne peut jamais être un départ, puisque le sélecteur « Début » de l'app n'offre que A et H.
- « CRÉDITS DE L'UNIVERSITÉ LAVAL » liste tous les crédits ULaval jamais obtenus, sans distinction de programme ; un certificat ou un bac antérieur y figure au même titre que le programme visé. `PROGRAMME(S) FRÉQUENTÉ(S)` (`parser::transcript::parse_program_floor`) donne le seul repère disponible : la plus ancienne date de « Fréquentation » parmi les programmes encore « En cours » (jamais un « Diplôme obtenu », qui signale un programme terminé) devient `program_floor`, et toute session antérieure est écartée de `earliest_start` — rapportée `OutsideHorizon` comme n'importe quelle session hors horizon, jamais silencieusement absorbée. Cette section est hors grammaire (comme « INFORMATIONS ÉTUDIANTES » ou le bilan) : son absence ou un format imprévu ne produit aucune anomalie, seulement `program_floor: None`, qui restaure l'ancien comportement non borné.
- Un été situé avant `start` (ou après la limite de l'horizon) n'est donc jamais un départ : ses cours sont rapportés `OutsideHorizon`, jamais silencieusement absorbés dans le placement.
- La présence d'au moins un été Laval/en cours à l'intérieur de l'horizon calculé ouvre `summers_open`.
- L'horizon (`study_sessions`) grandit au besoin, par une recherche bornée de 2 à 32 sessions, pour couvrir la session la plus tardive du relevé ; au-delà de 32, le surplus est rapporté `OutsideHorizon` plutôt que de repousser indéfiniment le plan.
- Principe directeur, sur tout le pipeline (parseur et application) : aucune ligne du relevé n'est avalée en silence — chaque cours atterrit dans exactement un de `pinned`, `credited` ou `ignored`, et chaque ligne HTML hors grammaire devient une anomalie visible.

## Alternatives rejetées

- **Parser le relevé dans la vue** : le parsing HTML et les règles de passage/reprise/horizon sont de la logique métier, testable indépendamment de Dioxus ; elle vit dans `core`, comme le parseur de pages ULaval.
- **Ignorer silencieusement les lignes inconnues** : un relevé mal formé ou une mise en page Capsule modifiée doit se voir, pas produire un plan tronqué sans explication.
- **Forcer une session de début fixe (ex. la session courante)** : un étudiant qui importe son relevé après plusieurs sessions doit retrouver son vrai point de départ, pas repartir d'aujourd'hui.
