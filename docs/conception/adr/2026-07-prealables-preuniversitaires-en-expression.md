# Le champ préuniversitaire est extrait comme expression, pas comme liste de sigles

Date : 2026-07-31

## Contexte

`BIO-1003` a produit la seule anomalie du scrape complet : « Malformed entry for préalables préuniversitaires : Préalables préuniversitaires nécessaires s'il y a lieu : BIO-0150, CHM-0150, CHM-0160 ou CHM-0170. », et ses préalables finissaient à `null` dans le snapshot.
Deux défauts dans l'extraction de l'ADR `2026-07-prealables-preuniversitaires-fusionnes` :

- `split_once(':')` coupait au **premier** `:` du nœud texte; le préfixe « REMARQUE : » de BIO-1003 décalait la coupe, le premier token devenait « Préalables » et la marche de sigles s'arrêtait à zéro code — d'où l'anomalie.
- Même sans préfixe, la marche s'arrêtait au premier token ni sigle ni « et » : dans « BIO-0150, CHM-0150, CHM-0160 ou CHM-0170 », le « ou » jetait `CHM-0170` **en silence** (aucune anomalie, puisque des sigles étaient déjà sortis) et le reste était joint par `ET`, sémantiquement faux.
- La page porte de plus **deux messages préuniversitaires différents** selon la section — la liste ambiguë ci-dessus et « (BIO-0150 ou BIO-NYA ou équivalent) ET (CHM-0160 ou CHM-0170 ou CHM-NYB ou équivalent) » — et seul le premier nœud trouvé était lu.

## Décision

La marche de sigles — une seconde grammaire ad hoc — disparaît. Chaque nœud marqueur livre l'**expression** entre le `:` qui suit « nécessaires » (immunisé contre « REMARQUE : ») et le premier `.` (fin de phrase, ou prose collée « MAT-0150.Cette section… »), connecteurs `et`/`ou` mis en majuscules, puis la grammaire des préalables existante s'en charge :

- ce qu'elle ne peut pas vérifier — listes à virgules (« BIO-0150, CHM-0150, CHM-0160 »), sigles de cégep (« BIO-NYA »), « équivalent » — survit en opérande `Raw` surfacée (ADR `2026-07-operande-non-verifiable-gardee-en-texte`), non bloquante pour le vérificateur (ADR `2026-07-prealable-inconnu-non-bloquant-remonte`);
- les messages **distincts** d'une même page sont tous gardés, dédupliqués puis ET-joints parenthésés avec le préalable régulier — rien n'est perdu, et la partie vérifiable vient du message le plus précis (décision de l'utilisateur, 2026-07-31);
- garde-fou conservé : une expression sans aucun sigle (« voir la direction »), ou un marqueur sans `:` exploitable, reste une anomalie `MalformedEntry` — jamais un abandon silencieux.

`BIO-1003` est gelé comme fixture (`tests/fixtures/test_cases/courses/bio-1003.{html,json}`).

## Alternatives rejetées

- **Corriger seulement le `:`** : la marche aurait extrait trois sigles et jeté « ou CHM-0170 » en silence — violation de « ne jamais perdre une entrée non reconnue ».
- **Interpréter les virgules** (`,` = `OU` selon l'énumération française, ou `,` = `ET` conservateur) : les deux messages de BIO-1003 se contredisent (le second exige BIO **ET** CHM, sans CHM-0150) — toute interprétation serait une invention; l'utilisateur a tranché pour `Raw` surfacé.
- **Ne lire que le premier message** (statu quo) : la version parenthésée, entièrement dans la grammaire, n'aurait jamais été vue.
