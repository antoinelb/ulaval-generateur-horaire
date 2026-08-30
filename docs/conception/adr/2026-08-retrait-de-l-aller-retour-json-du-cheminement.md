# Retrait de l'aller-retour JSON du cheminement

Date : 2026-08-30

## Contexte

L'interface offrait un aller-retour complet sur le cheminement en fichier `.json` : « Charger depuis JSON… » dans le panneau de gauche (ADR `2026-08-un-cheminement-par-fichier`) et l'entrée « JSON » du menu « Exporter ▾ » (ADR `2026-08-menu-d-export-unique`).

Le geste demande à l'étudiante de manipuler un document dont elle n'a aucune raison de connaître la forme : le « ? » à côté du bouton devait déplier un gabarit, expliquer `completed`, `sessions`, `semester`, `frozen`.
C'est le seul endroit de l'application où le format interne remonte à l'écran.
À côté, deux chemins font le même travail sans jamais nommer un format : « Partager » emporte l'organigramme entier dans un lien, et le tiroir Capsule entre un dossier réel par un collage.

Le menu d'export en souffrait aussi : trois entrées, deux entêtes de groupe et deux rangées libellées « PDF » pour dire deux documents.

## Décision

- **L'import et l'export JSON du cheminement disparaissent.**
  Le module `crates/ui/src/cheminement.rs` (983 lignes), le composant `CheminementLoader`, `present::present_cheminement_error` et `browser::download_text` — sans appelant une fois l'entrée du menu retirée — partent avec eux.

- **Le menu « Exporter ▾ » ne nomme plus de format** : deux entrées, « Organigramme » et « Horaire », chacune ouvrant son aperçu d'impression.
  Le champ `group` d'`ExportEntry` n'a plus rien à coiffer et disparaît ; `ExportChoice::OrganigrammePdf`/`HorairePdf` deviennent `Organigramme`/`Horaire`, un nom de document et non de format.
  Le libellé étant maintenant le nom du document, l'entrée s'aligne à gauche.

- **La classe `.panel-cheminement-help-toggle` est renommée `.panel-help-toggle`.**
  Le « ? » de la version de programme la portait aussi ; une classe nommée d'après une fonctionnalité retirée est un piège pour la prochaine lecture.

## Ce que le retrait coûte

Les 26 fichiers de `data/cheminements/` n'ont plus de consommateur dans l'interface : rien ne charge plus un cheminement type depuis un fichier.
Ils restent la source hors ligne des grilles officielles, écrits et vérifiés à la main, mais ne sont plus lisibles depuis l'application.

Ce qui subsiste couvre les deux besoins réels :

- **Sortir un organigramme et le reprendre plus tard** : « Partager » — le lien encode tout le document, y compris les cours manuels.
- **Entrer un dossier réel** : le collage Capsule.

Manque donc, jusqu'à nouvel ordre : la mise en place d'un cheminement type d'un seul geste.
Il faudra la rendre autrement — un choix dans la liste des programmes plutôt qu'un fichier à trouver sur un disque — si le besoin revient.

## Alternatives rejetées

- **Ne retirer que l'import, garder l'export JSON** : un format qu'on écrit et que plus rien ne relit est pire qu'une absence — il promet une reprise que l'application ne tient pas.

- **Garder l'aller-retour et supprimer seulement le « ? »** : le format resterait à découvrir par essai-erreur, ce que le codebase refuse ailleurs (« ne jamais laisser tomber une entrée en silence »).

- **Charger les cheminements types depuis les fichiers embarqués, sans sélecteur de fichier** : c'est la bonne réponse au besoin, mais c'est une fonctionnalité à concevoir, pas un reste de celle-ci ; elle demandera son propre ADR.

## Conséquences documentaires

`CLAUDE.md` est mis à jour (section `ui`).
Les ADR `2026-08-un-cheminement-par-fichier` et `2026-08-menu-d-export-unique` sont dépassés sur ce point ; ils gardent leur texte — ce sont des archives de décision — et portent désormais une ligne de renvoi vers celui-ci.
