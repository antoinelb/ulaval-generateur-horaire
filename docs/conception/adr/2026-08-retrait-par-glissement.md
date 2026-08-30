# Retirer un cours en le glissant sur la bande de statut

Date : 2026-08-30

## Contexte

Le glissement était asymétrique. Un sigle se tire d'une carte du ruban (`RibbonCode`) ou un bloc se tire de la grille (`GridBlock`), les deux portent le code dans `DraggedCourse`, et les deux n'atterrissent que sur une carte de session, qui les *déplace*.
Rien n'acceptait un dépôt qui *retire*.

Le seul retrait existant est le « ✕ » du panneau de gauche, qui exige d'aller sélectionner le cours dans la liste, et qui ne s'affiche pas pour un cours obligatoire (`choice != Not && !strip.mandatory`).
Un étudiant qui venait de tirer un cours dans une session devait donc changer d'outil pour défaire son geste.

## Décision

Pendant un glissement, **la bande de statut entière devient la zone de retrait**.
Un cours lâché là sort du cheminement — immédiat, annulable, jamais un dialogue (ACT-2).

La bande a été choisie parce qu'elle est déjà placée *entre* le ruban et la grille (`Shell`) : elle est sur le trajet des deux sources de glissement, et elle n'a aucune ressemblance avec une carte de session, donc « déplacer » et « retirer » n'habitent pas la même rangée.

Quatre arbitrages, tranchés avec Antoine :

- **Le calque est le frère du `role="status"`, jamais son enfant.** `Shell` monte `StatusStrip` et `RemovalDropZone` côte à côte dans un `.status-band` qui n'existe que pour donner au calque un repère de position. `.status-strip` garde exactement les enfants qu'il avait : insérer le calque dans la région vivante aurait fait annoncer son apparition comme un changement de statut.
- **L'accent dès le premier instant du geste**, pas seulement au survol : le retrait doit se découvrir sans qu'on l'ait cherché. Le survol appuie ensuite en bordure et fond pleins, sur `--accent-dark` — le blanc y tient 9,6:1, contre 5,9:1 sur le rouge vif.
- **Le calque recouvre toute la bande**, « ↶ Annuler » compris. Le masquage dure le temps où le bouton de la souris est enfoncé, et « Annuler » revient allumé *et nommé* — `undo_title` en fait « Annuler : GLG-1000 retiré » — à l'instant du lâcher. Rien à envelopper, aucun `pointer-events` à contourner.
- **Aucune icône.** Le mot porte seul le sens ; INP-3 est tenu par le texte, pas par un glyphe.

Le calque **nomme toujours le sigle**, y compris hors survol : « Retirer GLG-1000 du cheminement ».
L'objet d'un geste destructeur se dit, et un libellé qui changerait en cours de glissement se lirait moins bien qu'un libellé stable.

**Un cours obligatoire est refusé avant le dépôt.**
La bande s'affiche en gris et écrit « GCI-1000 est obligatoire au programme — il ne peut pas être retiré » ; `dragover` n'appelle pas `prevent_default`, donc le navigateur refuse le dépôt de lui-même (curseur « interdit ») et aucun `drop` ne part.
C'est la grammaire de `.ribbon-card--barred`, qui refuse sans rien pousser : le refus est déjà écrit à l'écran, il n'a pas à s'empiler ensuite en avis à chaque tentative.
Le prédicat est `panel::is_mandatory`, rendu public — celui-là même qui décide si le « ✕ » du panneau s'affiche, pour que les deux chemins de retrait ne puissent pas diverger.

L'entrée d'historique est `« {code} retiré »`, **mot pour mot celle du « ✕ »** : les deux chemins posent la même entrée, et « ↶ Annuler » la nomme pareillement.

Le glissement n'est ni un chemin clavier ni un chemin tactile ; le « ✕ » du panneau reste l'équivalent atteignable (INP-4) et n'est pas touché.

## Alternatives rejetées

- **Un emplacement réservé à côté de « Partager ».** C'était la proposition initiale. Elle tenait LAY-2 en réservant sa largeur en permanence, mais offrait la plus petite cible des quatre (~6,25 × 1,7 rem) et coûtait cette largeur à la bande même hors glissement. Le calque pleine bande donne une cible incomparablement plus grande pour zéro largeur perdue.
- **Une carte « Hors cheminement » au bout du ruban.** La moins coûteuse en code — elle réutilisait `SessionCard` et ses classes `--target` / `--landing` / `--barred` telles quelles — et le modèle mental le plus honnête (« aucune session » est une case comme les autres). Rejetée pour la raison décisive : elle plaçait une cible qui *supprime* en bout de rangée, voisine et sosie des cibles qui *déplacent*. Viser la dernière session et rater d'un centimètre aurait supprimé le cours, et un retrait qu'on ne remarque pas n'est pas annulé.
- **Une zone de retour dans le panneau de gauche.** La cohérence sémantique la plus forte — retirer et « ✕ » au même endroit — mais le trajet le plus long depuis le ruban, et une dépendance à un `position: sticky` dans le seul élément à défilement interne de l'application.
- **« Déposer n'importe où ailleurs » vaut retrait.** Zéro pixel, zéro dessin, et le piège classique : un glissement abandonné — la main qui hésite, la souris qui glisse — serait devenu une suppression. ACT-2 autorise le geste destructeur immédiat *parce qu'il est annulable*, pas parce qu'il est indevinable ; la cible doit être visible et visée.
- **Un « ✕ » sur le sigle du ruban, sans glissement du tout.** Le barreau le moins cher : un clic, aucune nouvelle grammaire de dépôt. Écarté parce qu'il ne règle pas ce qui était en cause — on tire pour placer, il aurait fallu cliquer pour retirer.

## Ce que ça touche

- `present::removal_band` — la décision et les deux libellés, purs et testés ; `crates/ui/src/components/` est hors couverture, la logique n'y vit donc pas.
- `components::header::RemovalDropZone` — le calque et ses trois gestionnaires, monté par `Shell`.
- `panel::is_mandatory` — passé en `pub`.
- `.status-band`, `.status-drop`, `--landing`, `--barred` dans `main.css`. Le `z-index: 25` passe au-dessus du menu d'export (20), qui peut être resté ouvert quand le glissement commence.
