# Le troisième cycle est hors périmètre

Date : 2026-07-19

**Amendé le 2026-07-20 par `2026-07-cycles-hors-perimetre-sans-erreur`** : la décision tient, mais la prémisse « ces activités portent un numéro `8xxx` » est fausse (PSY-7851 à 7854), donc le filtre du §4 est un raccourci avant requête HTTP et non une règle exhaustive. L'exclusion passe désormais par `Ok(None)` plutôt que par une erreur, et couvre aussi « Études post-MDD ».

## Contexte

La carte « Cycle du cours » de la page cours porte une **liste** de valeurs, pas une valeur unique : GEX-7002 annonce « Deuxième cycle » *et* « Troisième cycle ».
Le libellé lui-même varie en nombre — « Cycle du cours » sur cinq fixtures, « Cycles du cours » sur GEX-7002.

Or `core::Cycle` ne connaît que `First` et `Second`, et `TryFrom<u8>` rejette `3`.
Le JSON attendu de GEX-7002 retient `"cycle": 2` : la seconde valeur était donc écartée silencieusement, ce que la contrainte « ne jamais perdre une entrée sans le signaler » interdit.

Le domaine réel est plus étroit qu'il n'y paraît.
Un cours inscriptible à un horaire relève de `{1}`, `{2}` ou `{2, 3}`.
Les cours de **3e cycle seulement** sont les activités de recherche doctorale : des jalons administratifs occupant une session sans plage horaire, jamais un cours qu'on place dans une grille.
Ces activités portent un numéro `8xxx`.

## Décision

1. **Un cours conserve un `cycle` unique, le plus bas de la liste annoncée.**
   `{2, 3}` vaut donc `2` — le cycle auquel on s'inscrit.
   `Cycle { First, Second }` est inchangé ; aucune fixture ne bouge.
2. **Le parseur reconnaît « Troisième cycle » sans en faire une anomalie.**
   La liste est convertie en `u8` (`Premier` → 1, `Deuxième` → 2, `Troisième` → 3), le minimum est pris, puis converti en `Cycle`.
   Le `3` est transitoire : il ne survit pas au modèle, mais il est *reconnu*, donc son exclusion est délibérée et non une perte silencieuse.
   Toute autre chaîne reste une anomalie.
3. **Le libellé de la carte est reconnu par préfixe** (`starts_with("Cycle")`), le site alternant singulier et pluriel selon le nombre de valeurs.
4. **Les cours `8xxx` sont exclus du catalogue** à la construction (`Catalogue::from_entries`), le numéro étant lu après le tiret du sigle.
   Un cours dont le minimum vaudrait 3 est ainsi écarté en amont, avant même que sa page ne soit visitée : l'exclusion `8xxx` et la règle du minimum décrivent le même périmètre par deux chemins.

## Conséquences

`from_entries` filtre en plus de trier et dédupliquer ; un test épingle que `MAT-8000` disparaît quand `MAT-1800` survit — seul le premier chiffre du numéro compte.
Un sigle sans tiret est conservé : le filtre ne peut pas décider, il ne supprime donc pas.

Le catalogue produit est plus petit que le total annoncé par le site ; l'écart est voulu et distinct du bogue de comptage documenté dans `2026-07-le-catalogue-est-lunion-des-facettes`.

Effet de bord vérifié sur les 10 224 sigles de `data/catalogue.json` : les huit seuls codes qui violent la forme `XXX-9999` attendue par `is_course_code` (`PSY-88A1` à `PSY-88A7`, `PSY-88C9` — des lettres dans le numéro) commencent tous par `8` et disparaissent donc avec le filtre.
Le prédicat n'a aucun faux rejet sur le catalogue réel, ce qui autorise à traiter une forme inattendue comme une erreur franche plutôt que comme une tolérance.

## Alternatives rejetées

- **Ajouter `Cycle::Third`** : le modèle porterait un état qu'aucun cours retenu ne peut prendre, et chaque site d'appel devrait traiter un cas mort.
- **`cycles: Vec<Cycle>`** : fidèle à la source, mais l'interface et le solveur devraient de toute façon en choisir un ; le champ change de nom dans les six fixtures pour un gain nul.
- **Anomalie sur « Troisième cycle »** : correcte au sens de la contrainte, mais elle se déclencherait sur chaque cours de 2e-3e cycle du catalogue et noierait le journal d'erreurs sous un cas parfaitement connu.
- **Filtrer les `8xxx` à l'analyse de la page plutôt qu'au catalogue** : coûterait une requête HTTP par cours doctoral pour finir par le jeter.
- **Filtrer sur le cycle plutôt que sur le numéro** : exigerait de visiter la page pour connaître le cycle, alors que le sigle suffit et est déjà dans le catalogue.
