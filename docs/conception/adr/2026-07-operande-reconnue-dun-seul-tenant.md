# Le tokenizer des préalables découpe sur `(`, `)`, `ET`, `OU` et lit chaque opérande d'un seul tenant

Date : 2026-07-20

## Contexte

`tokenize_prereq_raw` découpait le texte sur les espaces et lisait mot à mot.
Or une opérande n'a aucune structure interne : seuls `(`, `)`, `ET` et `OU` en portent.
Le découpage sur les espaces perdait donc l'information et le code la reconstruisait :

- `operand_length`, qui retrouvait la fin d'une opérande que le découpage venait de dissoudre ;
- deux compteurs `skip` distincts, pour ne pas relire les mots déjà consommés par une reconnaissance multi-mots ;
- des lectures en avant `words.get(i..i + 5)` appariées à `["Crédits", "exigés", ":", n]` ;
- `strip_credit_bounds`, une passe préalable sur **tout** le texte, avant que quoi que ce soit ne sache où commencent les opérandes.

Cette dernière était le vrai défaut : une plage `dddd à dddd` suivie du mot « Crédits » à l'intérieur d'une opérande `Examen …` aurait été retirée avant que la règle « Examen » ne la voie.
Jamais observé, mais possible par construction.

## Décision

Le tokenizer accumule les mots dans une opérande courante et la vide à chaque séparateur (`flush_operand`), puis une fois à la fin.
`classify_operand` reconnaît alors l'opérande **entière** dans un `match` sur la tranche de mots, une forme par bras — examen, plage de cours, exigence de crédits bornée, exigence avec matière, exigence sans matière, sigle — et tout le reste est hors grammaire.
Le compteur `skip` disparaît : la mémoire de position, c'est le tampon.

`PrereqToken` n'a plus que cinq variantes, dont une seule opérande : `Operand(PrereqTree)`, le classement produisant directement la feuille.
Les trois gardes `if !expecting_operand { … "two operands in a row" }` dupliquées n'en font plus qu'une.

Bilan mesuré : le bloc du tokenizer passe de 169 à 140 lignes (dont une vingtaine de commentaires ajoutés), et quatre fonctions disparaissent — `strip_credit_bounds`, `collapsed_bound`, `is_course_range`, `operand_length`.
Le gain n'est pas d'abord dans le décompte : c'est la disparition des deux compteurs `skip` et de la passe préalable, donc du passage d'une opérande à l'autre, désormais impossible par construction.

Deux étiquettes d'erreur changent, le comportement observable non :
`GLG-1000 GLG-1900` était « two operands in a row » et devient une opérande que nulle forme ne lit ; le message cite désormais le texte fautif au lieu de décrire l'automate.
La garde « two operands in a row » reste atteinte par un groupe fermé suivi d'une opérande — `( GLG-1900 ) GLG-1000` — la seule opérande qu'un séparateur ne peut pas absorber.

## Alternatives rejetées

- **Garder la lecture mot à mot** : elle venait d'absorber une troisième sorte d'opérande (`Raw`) et une passe de réécriture ; la prochaine forme du catalogue — il y en aura, sur 10 000 cours — coûtait plus cher dans cette forme que dans l'autre.
- **Découper la chaîne sur `"ET"`/`"OU"` directement** : impossible, « ETHIQUE » serait coupé et les parenthèses arrivent collées aux sigles. Le découpage sur les espaces reste, mais comme lexer ; le tokenizer travaille sur les mots.
- **Chercher en avant jusqu'au prochain séparateur** (`operand_length` généralisée, `skip = len - 1`) : même conception, même `classify_operand`, mais un compteur à garder synchrone d'une longueur calculée ailleurs — exactement l'erreur d'indice que le remaniement supprime.
