# US-35 — Ouvrir la grille horaire d'une session

**Persona** : Marie-Pier, qui veut voir à quoi ressemblera sa semaine à l'automne.
**Intention** : visualiser l'horaire hebdomadaire des cours d'une session de sa grille.

## Préconditions

- Une session contenant au moins deux cours ayant un horaire publié.

## Scénario

1. Marie-Pier clique « Grille horaire de session ».
2. Une fenêtre s'ouvre sur la première session de la grille.
3. Elle change de session dans le menu de la fenêtre.
4. Elle revient à la fenêtre principale et déplace un cours.

## Résultats attendus

- La fenêtre affiche une grille de 8 h 30 à 21 h 30 par tranches de 30 minutes, du dimanche au samedi.
- Chaque cours occupe une case colorée fusionnée sur sa durée, portant son sigle, avec le titre en infobulle.
- Une légende sous la grille liste les cours affichés.
- Le menu de session contient toutes les colonnes de session de la grille principale.
- Toute modification de la grille principale rafraîchit la fenêtre si elle est encore ouverte.
- Une session sans cours à horaire publié affiche « Aucun cours avec horaire publié pour cette session. »

## Repères pour le test e2e

- Capturer la fenêtre avec `page.on('popup')`.
- `#session-sel option` reflète les en-têtes de session.
- `td.event-cell` contient les sigles attendus; `#legend .legend-item` en compte autant que de cours distincts.
- La fenêtre appelle `window.opener.collecterEvenementsSession(code)` : la fonction doit rester exposée sur `window`.

## Variantes et cas limites

- Un deuxième clic sur le bouton ne rouvre pas une fenêtre : il ramène le focus sur celle qui est ouverte.
- Fermer la fenêtre puis recliquer en rouvre une neuve.
- Les couleurs viennent des pastilles; à défaut, la fenêtre retombe sur une palette indexée par un hachage du sigle.
- Le titre de la fenêtre est figé sur « génie mécanique » : incorrect pour les autres programmes.
