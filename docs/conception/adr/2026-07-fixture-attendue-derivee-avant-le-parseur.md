# Une sortie attendue qui précède le parseur est dérivée, jamais régénérée

Date : 2026-07-20

Précise la portée du point 1 de `2026-07-fixtures-programmes-regenerees`.

## Contexte

`2026-07-fixtures-programmes-regenerees` §1 pose une règle catégorique :

> Les six fixtures attendues […] sont **produites par le parseur** à partir du HTML gelé, relues à la main contre la page, puis figées.
> Elles ne sont plus jamais écrites à la main : c'est l'écriture manuelle qui a produit la dérive.

Elle est juste, et sa cause est documentée : trois fixtures saisies à la main quatre jours avant le gel du HTML avaient perdu deux cours et fabriqué une contrainte.

Elle ne peut pourtant pas s'appliquer telle quelle au lot d'ADR du 2026-07-20 sur les cours (`2026-07-credits-variables-en-enum`, `2026-07-sections-en-combinaisons-valides`, `2026-07-cycles-hors-perimetre-sans-erreur`, `2026-07-sections-de-premier-niveau-par-ascendance`).
Ces quatre corrections partent de cas que le parseur **ne sait pas encore lire** : il rejette la page, ou il en produit une lecture fausse.
Lui demander la sortie attendue rend soit une erreur, soit précisément le résultat qu'on cherche à corriger.

Et si l'on attend la correction pour régénérer, le cas de test devient tautologique : il affirme que le parseur fait ce que le parseur fait.
C'est exactement ce qu'un cas de test écrit **d'abord** doit empêcher.

## Décision

La règle §1 vaut quand le parseur lit déjà la page. Elle se lit désormais :

1. **Le parseur lit déjà la page** → la sortie attendue est **régénérée** par le parseur, relue à la main contre la page, puis figée. Inchangé.
2. **La sortie attendue doit exister avant que le parseur sache la produire** → elle est **dérivée indépendamment** du HTML gelé, par une implémentation de référence distincte du code sous test, et cette implémentation doit d'abord **reproduire à l'identique les fixtures déjà validées**.

Pour ce lot, la dérivation est un script Python lisant les mêmes octets gelés selon les règles énoncées dans les quatre ADR.
Contrôle préalable : sur les quinze fixtures cours existantes, il en reproduit **treize au bit près**, et les deux écarts sont exactement `gci-1007` et `gci-2010` — les deux que `2026-07-sections-en-combinaisons-valides` prévoit de faire changer.
Une implémentation qui s'accorde avec treize pages déjà validées par le parseur n'est pas une transcription manuelle : c'est une seconde lecture de la source.

3. **Après la correction du parseur, les quatre sorties dérivées sont comparées à ce qu'il produit.**
   Un écart n'est pas résolu en régénérant : il est arbitré contre la page, et le perdant est corrigé.
   C'est le contrôle qui remplace la régénération, et il est plus fort — deux lectures indépendantes de la même source doivent converger.

4. **Le point 2 de `2026-07-fixtures-programmes-regenerees` — « un `.html` sans `.json` est un défaut » — souffre une exception nommée : la page hors périmètre.**
   `mdd-5101` et `psy-7851` sont gelées en HTML seul parce qu'il n'y a aucun `Course` à sérialiser (`2026-07-cycles-hors-perimetre-sans-erreur`).
   Leur sortie attendue est « rien », et c'est un test dédié qui la porte, pas un fichier.
   L'absence de `.json` y est le fait à épingler, pas un oubli.

## Conséquences

Six pages sont gelées dans ce changement : `med-1911`, `ift-1004`, `cso-6702`, `drt-7104` avec leur `.json` dérivé, `mdd-5101` et `psy-7851` sans.

Le lot est volontairement rouge à la compilation avant la correction du parseur.
Chaque erreur nomme un changement de production à faire — type `Credits` absent, champ `options` absent, `parse_cycle` et `parse` qui ne rendent pas d'`Option` — ce qui est la forme normale d'un cas de test écrit d'abord au travers d'un changement de type.

L'implémentation de référence n'est pas versionnée : son rôle s'arrête au contrôle du point 3, après quoi les fixtures figées font seules autorité.

## Alternatives rejetées

- **Régénérer les quatre `.json` avec le parseur corrigé** : conforme à la lettre de §1, mais le cas de test ne prouve plus rien — il enregistre le comportement obtenu au lieu de le contraindre. Aucune des quatre corrections n'aurait été mise en défaut par un tel test.
- **Écrire les quatre `.json` à la main** : c'est la faute que `2026-07-fixtures-programmes-regenerees` documente. IFT-1004 seul porte 11 sections et 7 options réparties sur trois saisons; la probabilité d'une transcription exacte est faible, et une erreur y serait indiscernable d'un bogue du parseur.
- **Attendre la correction et n'ajouter les fixtures qu'ensuite** : renonce à l'ordre imposé (le cas de test d'abord) et laisse le parseur être son propre juge.
- **Vérifier l'implémentation de référence sur les seules pages nouvelles** : elle n'aurait alors aucun point d'ancrage — c'est l'accord sur les treize fixtures déjà validées qui la rend crédible sur les quatre autres.
