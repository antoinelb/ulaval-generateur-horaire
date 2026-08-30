# La case du gel descend en pied de carte, dit « Gelé », et quitte l'été vide

Date : 2026-08-30

Révise l'ADR `2026-08-gel-en-case-a-cocher-dans-la-carte` (le lieu de la case, son glyphe, la case de l'été vide) ; s'appuie sur `2026-08-carte-de-session-chassis-et-face` (le châssis, qui ne change pas).

## Contexte

La case du gel a été posée la veille dans l'en-tête de la carte, entre le libellé et les crédits, étiquetée par le seul glyphe ❄, et la bande d'un été vide en a reçu une copie pour ne pas perdre l'affordance.
Antoine, à l'usage, tranche trois points :

1. « Le checkbox devrait être en bas à droite de la carte, et il devrait être écrit Gelé avant. »
2. « Il ne devrait pas y avoir de checkbox pour les étés qui n'ont pas de cours. »
3. « Le checkbox coché est rouge, ça attire trop le regard ; je le voudrais gris ou beige, similaire au restant de l'application. »

Ce que l'usage a montré : dans l'en-tête, la case s'insère entre les deux valeurs que l'on vient y lire — le nom de la session et ses crédits — et casse cette lecture au moment même où l'on balaie les huit cartes.
Le glyphe ❄ seul demande d'avoir appris ce qu'il veut dire ; il n'a jamais dit « gelé » à qui le rencontre.
Sur la bande d'un été vide, large de 1.75rem, la case était le seul élément horizontal d'une colonne verticale, et elle y coûtait une commande de plus pour un cas — geler une session qui ne porte rien — dont l'utilité est théorique.
Enfin la case portait `accent-color: var(--accent)` (#e30513, le rouge ULaval) quand toutes les autres cases de l'application, celles de `.panel-fit`, portent `var(--ink)` : une divergence, pas une convention.

## Décision

**La case du gel descend dans un pied de carte, alignée à droite, précédée du mot « Gelé ». L'été vide n'en porte plus. La case prend le neutre des autres cases de l'application.**

```
div.ribbon-card            châssis — bordure, hauteur, états, glisser-déposer
  div.ribbon-card-head     libellé, crédits
  button.ribbon-card-face  insignes, sigles, annotation
  div.ribbon-card-foot     label « Gelé » + case à cocher
```

- **Le pied, pas l'en-tête.** L'en-tête retrouve sa forme d'origine : deux valeurs, rien entre elles. Le pied est le seul endroit d'une carte que rien d'autre ne réclame, et « en bas à droite » est là où l'on pose une commande secondaire sans la mettre sur le chemin de la lecture.
- **Le pied reste hors du bouton.** Même raison qu'hier, inchangée : une case à cocher dans un `<button>` est du HTML invalide, son clic remonterait au bouton parent et l'ARIA ignore les descendants interactifs d'un `role=button`. `.ribbon-card-face` garde `flex: 1 1 auto`, ce qui pousse le pied contre le bas.
- **Le mot remplace le glyphe.** « Gelé » se lit sans apprentissage, tient dans la largeur d'une carte (5.5rem au minimum) là où l'en-tête n'avait la place d'aucun mot, et porte l'état avec la case cochée — la bordure en tirets de `.ribbon-card--frozen` reste en redondance, l'état ne tient donc jamais à la seule couleur (INP-3). Le ❄ disparaît de la carte ; il reste le préfixe du contenu d'un été gelé et l'étiquette du bouton « ❄ Tout dégeler » de la barre du haut, où il accompagne un mot.
- **Le nom accessible commence par le mot visible :** `present::freeze_toggle` renvoie désormais « Gelé — session A1-A26 » au lieu de « Geler la session A1-A26 ». Le nom entendu doit contenir le mot lu (WCAG 2.5.3) ; il ne bascule toujours pas avec l'état, et nomme la session ensuite puisque huit cases identiques se suivent. Le `title` et l'étiquette d'annulation (`act`) ne changent pas : la bascule reste un acte étiqueté et annulable par `edit_plan` (ACT-2).
- **La carte grandit de `10.75rem` à `12rem`.** Le pied coûte une ligne et sa gouttière (1.125 + 0.0625 = 1.1875), portant le calcul à 11.8125, arrondi à 12 — la marge de 0.1875 couvre la bordure de 0.1875rem que prend une carte survolée pendant un glissement, contre les 0.125 comptés. Les sept lignes de sigles sont le budget qu'Antoine a fixé la veille : c'est la carte qui grandit, jamais le corps qui se rogne.
- **L'été vide n'a plus de case.** Un été occupé s'affiche déjà comme une `SessionCard` ordinaire et garde donc la sienne. Un été vide reste gelable par « Tout geler » dans la barre du haut, et l'écran continue de le dire : le préfixe ❄ de son contenu vertical et le `title` qui porte le mot « gelée » restent (TRU-1, INP-3). Ce qui est retiré est une commande, pas une information.
- **La case cochée passe à `var(--ink)`**, le neutre de `.panel-fit input` — les cases « Ouvrir les étés » et « préalable en concomitance ». L'accent rouge est réservé à ce qui doit être remarqué, et geler n'est pas une alerte.

## Alternatives rejetées

- **Garder la case dans l'en-tête et n'y ajouter que le mot** : l'en-tête n'a pas la largeur de « Gelé » entre un libellé de session et un compte de crédits, et c'est précisément l'insertion entre ces deux valeurs qu'Antoine trouve trop voyante.
- **Loger le pied dans les 10.75rem existants en repassant à six lignes de sigles** : rognerait le budget fixé la veille sur la demande explicite « qu'elle puisse montrer 7 cours ».
- **Case à gauche, mot à droite** (l'ordre HTML habituel d'une case à cocher) : demandé dans l'autre sens, et « Gelé [x] » se lit comme une affirmation dont la case donne la valeur.
- **`var(--muted)` (#6f6a63, le gris chaud) plutôt que `var(--ink)`** : plus proche du mot « gris », mais ce serait un troisième neutre pour un contrôle de même nature que ceux du panneau, et le crochet blanc y perd du contraste (INP-2). La cohérence demandée désigne littéralement le neutre déjà employé.
- **Laisser sa case à l'été vide** : une commande de plus, dans la seule bande trop étroite pour la porter, pour geler une session qui ne contient rien ; « Tout geler » couvre le cas et l'écran le dit toujours.

## Réserve assumée, reconduite

L'étiquette cliquable mesure désormais ~46 × 18 px (4 px de padding, ~25 px pour « Gelé » à 12 px, 3 px de gouttière, 14 px de case, sur les 18 px de hauteur du pied) contre ~28 × 18 px hier : le mot élargit la cible de plus de moitié, sans atteindre les 48 dp d'INP-1. La densité du ruban — huit cartes de 5.5rem minimum sur une rangée — ne laisse pas la place d'une cible conforme sans reprendre la mise en page entière, et grandir la carte de 30 px de plus pour y parvenir serait un changement de gabarit que personne n'a demandé. Le clavier y accède normalement (INP-4). À rouvrir si le ruban est un jour repris.
