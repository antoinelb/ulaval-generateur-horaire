# Toute borne « dddd à dddd » d'une exigence de crédits est retirée du texte

Date : 2026-07-20

Remplace la décision de l'ADR `2026-07-credits-exiges-bornes-au-premier-cycle`, dont il ne reste que le constat de départ.

## Contexte

L'ADR précédent ne retirait que la borne du premier cycle (`1000 à 4999`) et laissait toute autre borne hors grammaire, par crainte qu'une exigence de deuxième cycle ne devienne silencieusement une exigence sur tous les crédits.
Le scrape de PHI a produit la forme attendue :

```
Parsing …/phi-7750-enseignement-individuel: Malformed prerequisites course code.: PHI-6000 à 8899, Crédits exigés : 12
```

L'expression entière basculait en `Prerequisites::Raw`.

Le cycle visé par la borne n'a pas besoin d'être encodé : il se déduit du cours qui la porte.
Un cours de premier cycle exige des crédits de premier cycle, un cours de deuxième cycle des crédits de deuxième cycle — et `cycle` est déjà dans le snapshot (`parse_cycle` retient le plus bas cycle affiché ; PHI-7750 sérialise `"cycle": 2`).
La borne est donc reconstructible au moment de la planification, pas de champ à ajouter.

## Décision

La borne `dddd à dddd` — nue ou collée à une matière — est lue **dans la forme complète de l'opérande**, `[borne, "à", borne, "Crédits", "exigés", ":", n]`, et n'en ressort que la matière : `ACT-1000 à 4999, Crédits exigés : 39` → `ACT` + 39 crédits, `1000 à 4999 Crédits exigés : 15` → aucune matière + 15 crédits, `PHI-6000 à 8899, Crédits exigés : 12` → `PHI` + 12 crédits.
Les mêmes trois mots sans « Crédits » derrière sont une autre forme (voir plus bas), d'où la reconnaissance de l'opérande entière plutôt qu'un retrait de texte préalable (ADR `2026-07-operande-reconnue-dun-seul-tenant`).
La logique d'horaire interprétera « N crédits » comme « N crédits du cycle du cours », sans lire la borne.

Deux conséquences assumées :

- **Une borne plus étroite que son cycle est élargie** (`1000 à 2999` sur un cours de premier cycle deviendrait « n'importe quel crédit de premier cycle »). La forme n'a jamais été observée — seuls `1000 à 4999` et `6000 à 8899` existent dans les données scrapées à ce jour — et le texte source reste dans `raw`, donc consultable.
- **Une borne qui déborde du cycle est resserrée** (`6000 à 8899` couvre le deuxième cycle et le bas du troisième ; lue comme « deuxième cycle » elle est plus étroite que la source). Un préalable refusé à tort plutôt que satisfait à tort : le sens sûr pour un solveur.

La borne n'est retirée que devant « Crédits » : les mêmes trois mots sans exigence de crédits derrière désignent une plage de cours, gardée en texte par l'ADR `2026-07-operande-non-verifiable-gardee-en-texte`.

## Alternatives rejetées

- **Garder la borne hors grammaire sauf `1000 à 4999`** (l'ADR précédent) : le mode d'échec revient à chaque matière de cycle supérieur, et la seule information perdue en l'acceptant est déjà dans `cycle`.
- **Ne retirer que les bornes couvrant tout un cycle** : il faudrait câbler les seuils de cycle dans le tokenizer, qui ignore tout du cours qu'il parse, pour distinguer une forme jamais observée.
- **Porter la borne dans `ProgramCredits`** (un champ `levels`) : déjà rejeté, et le cycle du cours la rend redondante.
