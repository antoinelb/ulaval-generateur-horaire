# Un cours au-dessus du deuxième cycle est hors périmètre, pas une erreur

Date : 2026-07-20

Amende `2026-07-troisieme-cycle-hors-perimetre`, dont la décision reste valide mais dont une prémisse est fausse.

## Contexte

Le premier scrape complet a produit 20 enregistrements en deux familles dans `data/cours_errors.log`.

**Seize `Malformed entry for cycle: Études post-MDD`.**
La carte de MDD-5101 lit `Cycle du cours | Études post-MDD` — un résidanat post-doctoral en chirurgie buccale et maxillo-faciale.
`cycle_level` ne connaît pas ce libellé et en fait une anomalie.

**Quatre `Malformed entry for cycle: invalid level : 3`.**
PSY-7851 à 7854, « Activité de recherche thèse », n'annoncent que « Troisième cycle »; le minimum vaut 3, et `Cycle::try_from(3)` échoue.

Cette seconde famille **falsifie une prémisse** de `2026-07-troisieme-cycle-hors-perimetre`, qui écrivait :

> Les cours de **3e cycle seulement** sont les activités de recherche doctorale […] Ces activités portent un numéro `8xxx`.

et filtrait sur cette base dans `Catalogue::from_entries`.
PSY-785x sont exactement ces activités — des jalons de thèse sans plage horaire — et elles sont numérotées `7xxx`.
Le filtre `8xxx` n'est donc pas exhaustif.

Les deux familles sont pourtant hors périmètre pour les mêmes raisons que le troisième cycle : ce sont des activités qui occupent une session sans entrer dans une grille horaire, et `core::Cycle` ne connaît que `First` et `Second`.
Le vrai problème n'est pas qu'elles soient rejetées, c'est qu'elles le soient **par une erreur** : le parseur n'a que `Ok` et `Err`, il ne sait pas dire « page comprise, délibérément sans objet ».

## Décision

1. **`cycle_level` reconnaît `« Études post-MDD »` et lui donne le niveau 4.**
   Comme le `3` de « Troisième cycle », ce niveau est transitoire : il ne survit pas au modèle, mais il est *reconnu*, donc son exclusion est délibérée et non une perte silencieuse.
   Toute autre chaîne reste une anomalie.

2. **`parse_cycle` rend `Result<Option<Cycle>, ParseError>`.**
   Un minimum annoncé `>= 3` vaut `Ok(None)` — hors périmètre — au lieu d'une erreur.
   `{2, 3}` vaut toujours `Some(Cycle::Second)` : GEX-7002 ne bouge pas.

3. **`parse` rend `Result<Option<CoursePage>, ParseError>`.**
   `Ok(None)` remonte jusqu'à `scrape_course`, qui rend `(None, Vec::new())` : aucun cours, **et aucune anomalie**.
   Le type porte le fait qu'une page valide peut légitimement ne rien produire; un `ParseError::OutOfScope` jamais journalisé l'aurait caché derrière un nom qui ment.

4. **Le filtre `0xxx`/`8xxx` de `Catalogue::from_entries` est conservé, requalifié en raccourci.**
   Il évite une requête HTTP quand le sigle suffit à décider, mais il n'est **pas** exhaustif : la règle de périmètre qui fait autorité est celle du cycle lu sur la page.
   Les deux ne décrivent plus « le même périmètre par deux chemins », contrairement à ce qu'affirmait `2026-07-troisieme-cycle-hors-perimetre` §4.

## Conséquences

Les 20 lignes disparaissent du journal sans que rien ne soit dissimulé : ces cours n'ont jamais eu leur place dans un horaire.

Ces pages ne sont pas mises en cache — rien à cacher — donc elles sont refetchées à chaque run.
Vingt requêtes sur ~10 000 : négligeable.

> **Révisé le 2026-07-20** par `2026-07-cache-du-verdict-hors-perimetre` : le verdict « hors périmètre » est désormais mis en cache sous l'empreinte de la règle de périmètre, si bien qu'une relance ne les refetch plus.
> Le raisonnement « rien à cacher » tenait pour le `Course` absent, pas pour le verdict lui-même.

`mdd-5101` et `psy-7851` rejoignent les fixtures gelées **en HTML seul, sans `.json`** : il n'y a pas de `Course` à sérialiser.
C'est la conséquence visible du point 3 — un cas de test qui n'a pas de sortie attendue est précisément ce que le type annonce.
Un test dédié les épingle ensemble.

Les douze sites d'appel de `parse` (tests compris) déroulent une option de plus.

## Alternatives rejetées

- **Une variante `ParseError::OutOfScope` que le scraper filtre** : aucun changement de signature, un seul bras de `match` à ajouter. Rejeté parce qu'une « erreur » qui n'est jamais journalisée et qui n'indique aucun défaut est un abus de nom, et parce que rien dans le type n'empêcherait un futur appelant de la traiter comme une anomalie.
- **Ajouter `Cycle::Third` et `Cycle::PostMdd`** : le modèle porterait deux états qu'aucun cours retenu ne peut prendre, et chaque site d'appel devrait traiter des cas morts. Déjà rejeté par `2026-07-troisieme-cycle-hors-perimetre`, et l'argument tient toujours.
- **Étendre le filtre du catalogue à `MDD-5xxx` et `PSY-78xx`** : économiserait 20 requêtes en câblant une liste de sigles à maintenir à la main, alors que la page porte déjà la réponse. Le filtre sur le premier chiffre reste défendable parce qu'il est une règle de numérotation, pas une énumération.
- **Signaler une anomalie de niveau « information »** : introduit un second canal de gravité dans `cours_errors.log` pour un cas dont on ne veut rien faire.
