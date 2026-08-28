# Le profil n'apporte jamais de crédits neufs

Date : 2026-08-28

## Contexte

Antoine a constaté 129/120 cr le 2026-08-27 en remplissant B-GCI + une concentration + le Profil développement durable.
Sémantique officielle vérifiée sur les pages ulaval.ca pour les six profils du répertoire : 99 cr de tronc + 6 cr d'« autres exigences » + 15 cr de concentration = 120 cr sans profil, et « le cheminement de 12 crédits s'intègre aux cours complémentaires ».
C'est une substitution, jamais une addition : les crédits du profil sont pris à même des cours à option qui comptaient déjà.
Cette prose vit dans `Rule.notes` (ADR `2026-07-notes-en-prose-conservees`) mais `credit_summary` ne la lisait pas — il ne connaissait que « en sus » et préparatoire, donc comptait le profil en plus du reste.

## Décision

`CreditSummary` gagne un champ `profile_only: u32`, exclu de `counted`, jamais avalé silencieusement.

Le classement d'un cours sélectionné, dans l'ordre déterministe du `BTreeSet` :

1. préuniversitaire → `preparatory` (inchangé) ;
2. en sus (`en_sus_codes`, priorité conservée) → `in_addition` (inchangé) ;
3. sinon, si le code appartient à `profile_codes ∖ elsewhere_codes` :
   - `profile_codes` = `mandatory` du profil choisi ∪ les codes de ses règles — `List` directe ou `Reference` résolue par `resolved_rule_courses` de `core`, un `Err` (référence cassée) ne produisant aucun code, jamais un code inventé ;
   - `elsewhere_codes` = `program.mandatory` ∪ codes des règles du tronc ∪ `mandatory` + codes des règles de la concentration **choisie** — un bloc non choisi n'abrite rien, comme `en_sus_codes` (décision d'Antoine du 2026-08-19) ;
   - s'il tient tout entier dans ce qui reste de `free_allowance` (la somme des `max` des règles `Keyword::Any` à contrainte `Credits` du tronc et de la concentration choisie), il est absorbé : il compte dans `counted`, et son crédit entier est retiré de `free_allowance` — jamais de décrément partiel ;
   - sinon il tombe dans `profile_only`, exclu de `counted` ;
4. sinon, `counted` comme avant.

Le header affiche un filet quand `counted > required` (`present::bac_credit_label`) : « ⚠ {counted}/{required} cr au bac … — au-delà des {required} cr du programme », composé avec la note « en sus » existante (`bac_credit_note`), jamais écrasée.
Le groupe du profil, dans le panneau, porte une note fixe (`PanelGroup.note`) rappelant que ses crédits sont pris à même les autres blocs.

L'allocation ignore une éventuelle contrainte `Course` d'une règle libre : aucune n'existe dans les données scrapées à ce jour.

## Vérification sur données réelles

Deux tests d'intégration (`crates/wasm/tests/credits_b_gci_a26.rs`) chargent le vrai `data/programmes/B-GCI-A26.json` et le vrai catalogue.

- Concentration « Eau et environnement » choisie, profil rempli des quatre cours que sa Règle 1 liste déjà, plus GBO-2040 (Règle 2) et DDU-1000 (absorbé par le seul cr libre du tronc) : `counted = 120`, `profile_only = 0`.
- Aucune concentration choisie, profil rempli de ses propres cours, 15 cr de complémentaires en substitut d'une concentration : sélection brute à 129 cr (le nombre d'Antoine), `counted = 120`, `profile_only = 9` (DDU-1000 seul tient dans le cr libre du tronc).

Fait notable, plus étroit que ce que le plan supposait : avec n'importe laquelle des trois concentrations du B-GCI choisie, `profile_only` ne dépasse jamais 3 cr (DDU-1000 seul), parce que la Règle 2 de chacune référence « Cheminement sans concentration — Règle 1 », qui liste déjà les cinq cours d'option du profil développement durable.
Le débordement réel n'apparaît que si le profil est rempli avant qu'une concentration ne le soit.

## Alternatives rejetées

- **Consommer `coverage_report`** : devient un `Err` dès qu'une règle est sur-sélectionnée, ce qui rendrait le compteur muet ; et il n'attribue rien aux règles `any`, qui sont précisément ce que le profil doit pouvoir absorber.
- **Soustraction forfaitaire de `credits_required` du profil** : fausse dès que le profil est rempli en tout ou en partie de cours déjà listés par la concentration — le cas normal, pas l'exception.
- **Attribution avec `max` dans `rules.rs`** : contredirait l'ADR des règles reportées sans contrainte et provoquerait un remous de fixtures pour un calcul qui n'a jamais eu besoin d'y vivre.
