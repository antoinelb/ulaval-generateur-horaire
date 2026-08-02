# Le code de programme devient le code officiel du répertoire, le slug un champ dédié

Date : 2026-08-02

Remplace `2026-07-programs-sans-url-rafraichit-par-slug`.

## Contexte

`Program.code` portait le slug d'URL (`baccalaureat-en-genie-des-eaux`) parce que le parser croyait que la page n'écrivait son code officiel nulle part.
C'est faux : le code du répertoire (`B-GEX`, `MM-GEX`, `B-ANT`) apparaît dans les ids des boutons d'accordéon « Avenir » / « Poursuite des études » (`<button class="accordeon-oe-programme" id="B-GEX-GEX-avenir">`), vérifié sur les six pages gelées et une page live.
Le projet abrège tout type de diplôme à sa première lettre (`B`, `M`, `C`), par cohérence avec les codes de saison à une lettre (`A`/`H`/`E`).
La forme est `{code}-{matière}-{section}`, la matière répétant le dernier segment du code — les accordéons de structure partagent la classe mais leurs ids sont des slugs en minuscules (`genie-des-eaux-regle-1`).
Le code officiel est l'identifiant public stable d'un programme ; le slug n'est que son adresse web.

## Décision

- **`Program.code` = code officiel**, extrait des ids par un scan de segments : la première paire de segments doublés (tous deux majuscules alphanumériques, une section devant suivre) ferme le code.
  Zéro candidat = échec dur de la page (`MissingElement`) ; des candidats divergents = `MalformedEntry` nommant tous les codes — jamais de choix silencieux ni de repli sur le slug, car le code nomme le fichier snapshot.
- **Le préfixe de diplôme est normalisé à sa première lettre** : `MM-GEX` sur la page devient `M-GEX` (décision d'Antoine, 2026-08-02).
  Risque accepté : deux codes officiels distincts ne différant que par le préfixe (maîtrise avec mémoire `MM-` vs professionnelle `M-`) se replieraient sur le même code normalisé ; aucun cas concret dans le périmètre actuel, à réévaluer si un tel doublon entre au répertoire.
- **`Program.slug` : nouveau champ requis** (pas de `serde(default)`), le dernier segment du lien canonique — un snapshot sans slug doit échouer bruyamment au refresh, pas construire une URL bidon.
- **Fichiers renommés `{code}-{semestre}.json`** (`B-GEX-A26.json`), fixtures de test comprises (`tests/fixtures/test_cases/programs/B-GEX.{html,json}`).
- **Le refresh sans URL lit le slug dans le contenu** de chaque `*.json` (hors `*.manuel.json`), plus jamais dans le nom de fichier — la forme du suffixe de millésime devient indifférente au refresh.
  Un fichier illisible ou sans `slug` est une erreur dure nommant le fichier : un programme sauté en silence cesserait d'être rafraîchi sans que personne le sache.

## Alternatives rejetées

- **Un regex glouton ancré en tête** (`^([A-Z-]+)-([A-Z]+)-`) : mal-découpe `B-GEX-GEX-M-CNAM` (id réel), fatal sur la maîtrise dont c'est le seul id porteur ; et le workspace n'a pas de dépendance `regex`.
- **`slug` optionnel avec défaut** : un refresh sur un fichier sans slug fabriquerait une URL vide au lieu d'échouer.
- **Garder le slug comme `code` et dériver une abréviation à l'affichage** : le code officiel est une donnée de la page, pas une convention à inventer ; le perdre obligerait à le re-déduire.
- **Garder le préfixe de diplôme tel quel** (`MM-GEX` verbatim) : évitait le repli de codes distincts, mais rompait la convention « première lettre seulement » du projet ; la cohérence des abréviations l'a emporté, le risque de collision étant théorique dans le périmètre actuel.
