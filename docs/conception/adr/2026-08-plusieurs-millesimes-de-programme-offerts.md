# Plusieurs millésimes d'un programme, tous offerts au choix

Date : 2026-08-13

## Contexte

Le dédoublonnage des programmes traitait la coexistence de `B-GEX-A24.json` et `B-GEX-A26.json` comme une anomalie.
Antoine (note 5, 2026-08-13) : plusieurs millésimes ne sont pas une erreur — un étudiant fait son horaire sous la version du programme de son admission.
Vérification faite : les deux fichiers B-GEX diffèrent réellement (`mandatory`, `rules`) ; seul le champ `semester` du fichier A24 portait « A26 » par erreur, ce qui les faisait entrer en collision.

## Décisions

- Le champ `semester` de `data/programmes/B-GEX-A24.json` est corrigé à `A24` : deux vrais millésimes, deux entrées du sélecteur.
- Le sélecteur de programme liste chaque `(code, millésime)` — « B-GEX - version A24 - 120 cr » — trié par code puis millésime (tri ajouté dans `parse_data`) ; toute correspondance dans l'app se fait sur le couple `(code, semester)` (l'en-tête cherchait par code seul).
- Le dédoublonnage **reste**, restreint à son vrai cas : deux fichiers portant le même `(code, semester)` interne — là, le fichier dont le nom concorde gagne et l'ignoré est nommé.

## Alternative rejetée

- Supprimer le fichier A24 : il porte des règles distinctes dont les étudiants admis en A24 ont besoin.
