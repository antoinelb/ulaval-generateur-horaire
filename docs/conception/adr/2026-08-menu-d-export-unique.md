# Un seul menu « Exporter » dans la bande de statut

Date : 2026-08-29

> Dépassé en partie le 2026-08-30 par l'ADR `2026-08-retrait-de-l-aller-retour-json-du-cheminement` : l'entrée « JSON » et les entêtes de groupe ont disparu.
> Le menu ne porte plus que deux entrées, « Organigramme » et « Horaire ». Tout le reste — un seul contrôle, table pure, ouverture au clic, fermeture différée, surcouche ancrée — tient toujours.

## Contexte

La bande de statut portait deux boutons — « Exporter l'organigramme » et « Exporter l'horaire » — qui ouvraient tous deux l'aperçu d'impression.
L'organigramme gagne un second format, le JSON de cheminement (ADR `2026-08-un-cheminement-par-fichier`) : à trois destinations, trois boutons côte à côte, la bande déborde.

Le commentaire LAY-2 de `main.css` note que cette ligne retombe déjà à deux rangées vers 1280 px quand un cours hors grille, un conflit et une session forcée coïncident.

## Décisions

- **Un contrôle, « Exporter ▾ »**, ouvrant un menu de trois entrées groupées par document : Organigramme (PDF, JSON), Horaire (PDF).
  La variante à deux menus — un par document — laissait à l'horaire un menu d'une seule entrée.

- **La table du menu est pure** : `export::menu::entries()` donne les rangées, leur libellé et leur clé ; la vue n'en fait qu'un `for` (AP-5).
  Les deux rangées portent le libellé « PDF » : seule la clé les distingue (AP-8).

- **Le format seul, aligné à droite sous son entête de groupe.**
  Une glose grise à côté de chaque format (« aperçu d'impression », « cheminement rechargeable ») répétait ce que l'entête disait déjà et faisait passer « JSON » sur deux lignes.

- **Ouverture au clic, jamais au survol** (INP-5) ; `Échap` referme, comme partout ailleurs dans l'application ; `aria-haspopup` et `aria-expanded` sur le bouton.

- **La fermeture au `focusout` est différée d'un macrotask, et gardée par un compteur de `focusin`.**
  Refermer sur-le-champ mangeait le clic : cliquer une entrée retire d'abord le focus du bouton, ce qui démontait l'entrée avant que son `click` ne parte — le menu se refermait sans rien exporter.
  Un macrotask plus tard, le clic a été distribué.
  Le compteur distingue « le focus a quitté le menu » de « le focus s'est déplacé dans le menu » : sans lui, tabuler du bouton vers une entrée refermerait le menu (INP-4).

- **Le menu est une surcouche ancrée** sous le bouton (`position: absolute`) : ni la bande ni la grille ne bougent quand il s'ouvre (LAY-1).

- **Un téléchargement refusé n'est jamais annoncé comme réussi.**
  `browser::download_text` dit si le navigateur a pris le fichier ; `export::menu::download_note` en tire la phrase.
  Un succès est un ✓ qui s'efface, un refus une note qui reste (TRU-1, ALR-4).

## Alternative rejetée

- **Garder « Exporter l'horaire » en bouton simple et ne donner un menu qu'à l'organigramme** : le diff le plus court et aucun clic perdu sur le chemin fréquent, mais deux contrôles voisins de formes différentes pour deux actions jumelles.
