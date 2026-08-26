# La couleur d'un cours vient de sa matière, en oklch, pas de sa position dans l'horaire

Date : 2026-08-26

## Contexte

La couleur d'une carte de cours dans la grille horaire hebdomadaire venait de `color = index_dans_l_horaire % 6`, une position de boucle sur `schedule.report.courses`, mappée vers six classes CSS et douze couleurs hex figées.
Deux cours sans rapport (`STT-1900` et `CHM-1903`, observés côte à côte dans un horaire réel) finissaient avec la même teinte dès qu'ils tombaient sur le même reste modulo 6 : la couleur dépendait de la position dans l'horaire d'un étudiant donné, jamais du cours lui-même, et seules six teintes existaient quelle que soit la taille du catalogue.
Cette référence était déjà attendue ailleurs dans le dépôt : `tests/user_stories/86-b-gph-concentration-genie-des-materiaux.md` cite « l'ADR `2026-08-couleurs-derivees-de-la-matiere` » sans que ce fichier existe — une référence brisée, laissée par la rédaction des user stories avant que la décision soit écrite ici.
`tests/user_stories/53-couleurs-et-redimensionnement.md` documente un principe voisin (teinte fixe par matière, clarté et chroma constants) pour un composant distinct qui n'existe pas encore dans ce dépôt Rust : les « pastilles » d'un éditeur d'organigramme par glisser-déposer.
Cette décision ne construit pas cet éditeur ; elle s'applique à la grille horaire hebdomadaire, seule concernée aujourd'hui.

## Décision

La teinte d'un cours vient de sa matière (le préfixe du code, `STT`, `CHM`, `GCI`…) : rang alphabétique de la matière parmi toutes les matières distinctes du catalogue courant, divisé par leur nombre total, multiplié par 360 — `panel::subjects(snapshot)` donne déjà cette liste triée (`BTreeMap`) sur tout `snapshot.courses`, sans nouvelle énumération à écrire.
Deux matières voisines dans l'alphabet peuvent obtenir des teintes proches ; c'est accepté, pas un défaut à corriger par un hachage qui rendrait l'attribution imprévisible.
Le rang dépend du catalogue chargé : ajouter une matière avant une autre dans l'alphabet décale le rang — et donc la teinte — de toutes celles qui la suivent au prochain rechargement. C'est une conséquence acceptée du choix alphabétique, pas un bogue.
Un cours dont la matière n'est plus dans le catalogue (répertoire retiré, cf. `tests/user_stories/11-millesime-anterieur-et-cours-retire.md`) retombe sur une teinte à 0°, un cas géré et testé plutôt qu'un panique.

La teinte est en OKLCH : clarté (`45%`) et chroma (`0.12`) fixes, seule la composante teinte varie — `oklch(45% 0.12 <teinte>)` pour le texte du titre et la bordure gauche, la même couleur en transparence (`/ 12%`) pour le fond de la carte plutôt qu'un second jeu de couleurs pastel figées.
Ce fond transparent est posé sur un aplat blanc opaque : sans lui, la transparence se compose contre les lignes d'heure de la colonne du jour, qui restent alors visibles derrière chaque carte.
Le raccourci `background` n'admet une couleur unie que dans sa dernière couche — `background: oklch(...) / 12%, #fff` est invalide (deux couleurs) et se fait purement ignorer par le navigateur, d'où un fond totalement absent au premier essai plutôt qu'une transparence ratée.
La teinte passe donc par `background-image: linear-gradient(oklch(...) / 12%, oklch(...) / 12%)` (un dégradé aux deux arrêts identiques, donc plat) posé sur `background-color: #fff`.
Fixer clarté et chroma est l'intérêt propre de l'OKLCH par rapport au HSL : le contraste entre le texte et son fond reste constant sur toute la roue des teintes, alors qu'une même clarté HSL paraît beaucoup plus sombre pour un bleu que pour un jaune.
`Block.hue: f32` (dans `crates/ui/src/present.rs`) remplace `Block.color: usize` ; la teinte se lit en degrés directement dans le style calculé (`--course-h`), une classe unique `grid-block` remplace les six classes `grid-block--c0`..`c5`.
La vue imprimée demi-page garde son trait de bordure seul, sans aplat (économie de toner) : son fond `#fff` opaque, déjà présent pour une autre raison (empêcher les blocs voisins de transparaître), l'emporte sur la carte de spécificité et masque de toute façon le fond en transparence du même sélecteur en écran.

Portée : la grille horaire hebdomadaire seulement.
Le composant « pastilles » de `tests/user_stories/53-couleurs-et-redimensionnement.md` n'existe pas encore dans ce dépôt et calcule son rang sur les seuls sigles du fichier de programme, pas sur le catalogue entier, avec un fond pastel opaque plutôt qu'en transparence — une formule différente, à réconcilier avec celle-ci quand ce composant sera construit.

## Alternatives rejetées

- **Hachage du sigle** — teinte imprévisible d'une matière à l'autre ; l'utilisateur préfère l'ordre alphabétique, quitte à ce que des matières voisines se ressemblent.
- **HSL avec teinte variable** — une clarté fixe en HSL ne garantit pas un contraste constant entre teintes, contrairement à l'OKLCH, alors que la teinte est justement dessinée en avant-plan sur le texte du titre.
- **Fond pastel opaque, comme les pastilles (US-53)** — rejeté ici au profit d'un survol en transparence de la même couleur que le texte, sur demande explicite ; réduit le nombre de constantes à une seule couleur de base plutôt que deux paires figées.
