# Le nombre de crédits d'une carte dit d'où il vient

Date : 2026-08-30

S'appuie sur `2026-08-carte-de-session-tronquee-en-lignes-entieres` (le budget de lignes du corps, qui ne change pas) et `2026-08-plafond-par-defaut-17-credits` (le plafond signalé).

## Contexte

Antoine rapporte un bogue sur B-GMC : une session A7-A29 portant cinq sigles affiche « 21 ⚠ », et il soupçonne des crédits fantômes laissés par un déplacement de cours.

Il n'y en a pas. `data/cours.json` donne `GMC-2580` « Stage en génie mécanique II » à 9 crédits ; 3 + 3 + 9 + 3 + 3 = 21, et `crate::solve::session_credits` somme exactement ces `credits.planning()`. Le calcul est juste ; c'est l'écran qui ne permet pas de le vérifier.

Deux choses le rendent illisible :

- la carte ne montre que des sigles, jamais leur poids — le budget de sept lignes du corps ne le permettrait pas sans rogner les sigles eux-mêmes ;
- un cours peut peser 9 crédits, ou un intervalle compté à son plancher, ou zéro s'il est hors catalogue. Rien de tout cela ne se devine d'une liste de sigles.

Devant un nombre qu'on ne peut pas refaire de tête, la lecture par défaut est « le logiciel s'est trompé ». C'est exactement le diagnostic qui a été posé.

## Décision

**`RibbonCard` porte un champ `credits_detail` : la somme épelée cours par cours, servie en infobulle du nombre.**

```
GIF-4101 : 3 cr
GLO-4001 : 3 cr
GMC-2580 : 9 cr
GMC-3020 : 3 cr
PHI-3900 : 3 cr
Total : 21 cr, au-dessus de votre plafond de 17 cr
```

- **Le texte est calculé dans `present::credits_detail`, pas dans la vue** — `components/ribbon.rs` ne fait que poser l'attribut. La chaîne est du texte à décider, donc du modèle, et elle se teste nativement.
- **La liste est entière**, celle de `state::session_codes`, pas les sigles rognés par `ribbon_body` : l'infobulle existe précisément pour les cartes chargées, celles qui rognent.
- **Le cours hors catalogue est nommé** (« ZZZ-9999 : hors catalogue, 0 cr ») plutôt que taire : il pèse zéro dans la somme, et une somme dont un terme manque à l'écran est la même énigme qu'avant (« jamais rien perdre »).
- **Le plancher est dit quand il s'applique** : un stage à intervalle entre par son minimum, l'infobulle écrit « Total : 9 cr au minimum » et la ligne du cours garde son intervalle entier « 6–12 cr ». Sans quoi les lignes ne sommeraient pas au total (TRU-1).
- **Le plafond est rappelé sur place** quand il est franchi. Le ⚠ dit qu'il y a un dépassement ; il ne disait pas de combien ni par rapport à quoi.
- **Une session vide n'a pas d'infobulle** : son nombre est « — », il n'y a pas de somme à épeler.

## Sur INP-5 (pas d'affordance au survol seul)

L'infobulle n'est pas la seule voie vers l'information et n'est pas une commande : le panneau d'une session donne déjà les crédits de chaque cours par `present::credits_label` (`panel.rs:1973`). Le survol raccourcit un chemin qui existe ; il n'en ouvre pas un exclusif. Même usage que le `title` de l'insigne « ⚠ conflit d'horaire », posé au même endroit.

## Alternatives rejetées

- **Sortir les crédits « en sus » du plafond de session** (le stage porte `credits_in_addition: true` dans `B-GMC-A26.json`, donc ses 9 crédits ne comptent pas vers les 120 du diplôme) : le plafond d'une session mesure une charge, pas une progression. Un stage de quatre mois *est* la charge de sa session ; l'en retirer ferait afficher « 12 » à une session qui n'a pas de place pour autre chose. Ce sont deux questions différentes, et le bandeau d'en-tête répond déjà à la seconde.
- **Écrire le poids à côté de chaque sigle dans la carte** : mangerait le budget de sept lignes fixé le 2026-08-29, sur toutes les cartes, pour un nombre qui n'intrigue que sur celles qui portent un stage.
- **Ne rien changer** : le calcul est juste, mais il a été rapporté comme bogue par la personne qui connaît le mieux le domaine. Un nombre invérifiable est un défaut d'interface même quand il est exact (TRU-1).
