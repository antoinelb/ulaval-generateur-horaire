# Échec d'une page programme : anomalie, pas arrêt

Date : 2026-07-20

## Contexte

Le run `courses` traite une page injoignable ou non reconnue comme une anomalie et continue : à ~10 req/s, un scrape complet dure ~17 min, et une seule page ne doit pas jeter tout le reste (ADR `2026-07-echec-de-page-cours-non-bloquant`).

`ulaval-scraper program` est dans une position différente : l'utilisateur nomme lui-même les N URL.
On pourrait donc arguer qu'une URL en échec doit arrêter la commande — il a demandé N programmes, en livrer N−1 sans qu'il le remarque est un piège — et que relancer ne coûte que quelques secondes, pas 17 minutes.

## Décision

Même politique que `courses` : une URL en échec est une anomalie, jamais un arrêt.

1. Une URL injoignable produit un `ProgramError::Fetch` (le `FetchError` nomme déjà l'URL) ; une page dont le squelette manque produit un `ProgramError::Parse` portant l'URL — `ParseError` ne nomme qu'un sélecteur, la ligne de log ne serait pas actionnable sans elle.
   Dans les deux cas aucun fichier n'est écrit pour ce programme, et les autres sont écrits normalement (`collect`, jamais `try_collect`).
2. Un programme parsé **avec** des anomalies (prose hors grammaire, groupe sans `<h3>`) est écrit **et** journalisé : la page est lisible, ses trous sont signalés.
3. Le garde-fou est `write_error_log`, qui écrit `data/programmes_errors.log` **et** avertit sur la sortie d'erreur (« There were N anomalies. See … »), puis efface un log périmé quand le run suivant est propre.
   C'est lui qui rend cette politique tenable : rien dans `data/programmes/` ne signale une absence, seul le log le fait.

## Alternatives rejetées

- **Erreur bloquante, rien n'est écrit** : cohérent avec l'idée que la liste d'URL est un contrat explicite, mais fait perdre les programmes parfaitement lisibles à cause d'une URL mal tapée, et interdit le cas courant « scraper les six programmes, l'un d'eux étant momentanément en 503 ».
- **Erreur bloquante seulement sur `Fetch`, anomalie sur `Parse`** : distinction sans conséquence pratique pour l'utilisateur — dans les deux cas il manque un fichier — et une règle de plus à retenir.
