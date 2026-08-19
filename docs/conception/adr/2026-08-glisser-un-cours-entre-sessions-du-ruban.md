# Glisser un cours entre sessions du ruban

Date : 2026-08-19

## Contexte

L'ADR `2026-08-glisser-un-cours-vers-une-session` n'a livré que la moitié du geste prévu dès la conception initiale (« glisser-déposer des cours entre sessions ») : un bloc de la grille hebdomadaire se glisse vers une carte du ruban, mais les codes affichés dans les cartes sont inertes.
Antoine demande l'autre moitié ; son essai réel (2026-08-19) révèle en plus que le dépôt n'aboutissait pas dans un vrai navigateur, que le marquage par sonde estompait presque toutes les cartes, et qu'aucun indice ne montrait la carte survolée.

## Décision

- Chaque code d'une carte (`RibbonCode`) devient `draggable` et réutilise le circuit existant : le code voyage dans le signal `DraggedCourse` et le dépôt appelle `place_course` — un seul pas annulable.
- **Un jeton est tout de même écrit dans `DataTransfer`** (`set_data("text/plain", code)` + `effectAllowed: move`) : Firefox refuse de mener un glissement au `DataTransfer` vide. Le signal reste le payload que le dépôt lit — le jeton est le péage du navigateur, pas le transport. (Précise le « jamais `DataTransfer` » de l'ADR précédent.)
- **Les gestionnaires `dragover`/`drop` lisent les signaux au moment de l'événement**, jamais des valeurs capturées au rendu : la capture précède le glissement et gelait le refus/accueil dans son état d'avant-geste.
- **Le marquage suit le filtre de saison des puces** (`panel::offered_sessions`, pur et instantané), et remplace la sonde `admissible_sessions` de l'ADR précédent : une session dont la saison offre le cours garde son opacité et se marque cible ; les autres s'estompent et refusent le dépôt. La sonde mettait des secondes à répondre et barrait presque tout (plafond de crédits, préalables) — la vérification automatique juge de toute façon le résultat après coup, comme pour les puces. La machinerie côté ui (`request_admissible`, cache `SolverState.admissible`, `WorkerAnswer::Admissible`) part avec ; le module wasm garde `admissible_sessions` pour sa surface JS.
- **La carte survolée pendant un glissement se marque d'une bordure pleine plus foncée** (`--landing`, signal partagé `DropHover` posé par `dragover`, levé par `dragleave`/`drop`/`dragend`) — l'indice « le cours atterrirait ici », pour les deux origines (grille et ruban).
- La garde « déjà épinglé là » des puces remonte au début de `place_course` : déposer un cours sur la session où il est déjà épinglé ne fait rien, au lieu d'empiler une entrée d'annulation vide.
- Parité stricte avec les puces (INP-4) : sessions passées sources et cibles comprises, cours `manual` compris ; le glissement reste un raccourci souris, jamais le seul chemin ni un chemin plus permissif.

## Alternatives rejetées

- Garder le marquage par sonde `admissible_sessions` : des secondes de latence pour un verdict plus strict que le chemin clavier — l'asymétrie souris/clavier violerait INP-4, et l'estompage quasi général rendait le marquage illisible (retour d'Antoine).
- Transporter le code par `DataTransfer.getData` : illisible pendant `dragover` dans plusieurs navigateurs ; le signal reste le payload, seul le jeton minimal est écrit.
- Marquer le survol par `:hover` en CSS : les navigateurs gèlent `:hover` pendant un glissement natif — il faut l'événement `dragover`.
- Restreindre les sources aux sessions non passées : les puces permettent déjà d'épingler vers et hors de n'importe quelle session de l'horizon ; la vérification automatique juge le résultat.
