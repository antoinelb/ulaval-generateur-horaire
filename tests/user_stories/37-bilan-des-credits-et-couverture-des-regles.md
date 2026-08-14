# US-37 — Bilan des crédits et couverture des règles

**Persona** : Éliane, qui veut savoir combien de crédits il lui reste et quelles règles ne sont pas comblées.
**Intention** : lire le bilan en bas du journal.

## Préconditions

- Programme « B-GEX » A26 : 120 crédits exigés, 34 cours obligatoires, cinq règles, une règle « Stages » et une règle « Scolarité préparatoire ».

## Scénario

1. Éliane charge le cheminement type.
2. Elle retire deux cours de la Règle 3.
3. Elle lit le bilan.

## Résultats attendus

- Le bilan est regroupé par section (« Activités communes », puis la spécialisation), dans l'ordre d'apparition des règles.
- Chaque règle affiche `titre : X cr. / Y cr.`, ou `X cr. / (min à max cr.)` quand les bornes diffèrent.
- Une règle dont les crédits accumulés sont sous son minimum est en avertissement; sinon en information.
- Les crédits d'une règle sont plafonnés à son maximum : un surplus dans une règle ne comble pas une autre.
- La contribution d'une section est plafonnée par les crédits exigés de la section.
- La ligne finale `Total : X cr. / Y cr.` exclut la scolarité préparatoire et la règle « Stages ».
- Le bilan est recalculé après chaque modification de la grille.

## Repères pour le test e2e

- `#log-content .log-section` marque les en-têtes de section; la dernière `.log-entry` commence par `Total :`.
- Retirer un cours de 3 crédits fait baisser sa règle de 3 et le total de 3.
- Une règle non comblée porte la classe `log-warning`.

## Variantes et cas limites

- Une contrainte exprimée en **nombre de cours** est convertie en crédits avec les crédits des cours qu'elle offre, plafonnée à leur somme : la conversion est approximative et doit rester plafonnée.
- Une règle sans contrainte (la scolarité préparatoire) exige la somme de ce qu'elle liste.
- Une règle dont les cours valent `any` n'a aucune liste : ses crédits accumulés restent à zéro même si l'étudiant a bien fait un cours au choix.
- Un cours appartenant à deux règles compte dans les deux : le total global peut alors dépasser la réalité.
- Un cours placé mais absent de toute règle ne compte nulle part.
