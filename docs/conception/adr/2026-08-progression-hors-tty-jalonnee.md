# La progression hors TTY sort en lignes permanentes jalonnées

**Date :** 2026-08-09
**Statut :** accepté (décision Antoine).

## Contexte

Dans le CI, le scrape des ~8855 cours (~20 min) n'affichait rien entre « Scraping 8855 courses... » et sa ligne de fin : un log muet pendant vingt minutes, impossible de distinguer un scrape qui avance d'un scrape bloqué.

L'hypothèse d'un tampon non vidé (les `print!` sans fin de ligne) est fausse : `print::write` est le seul point qui touche stdout et vide le tampon après chaque écriture.
La cause réelle est un choix d'affichage : la ligne du bas (le compteur `done/total`) est **transiente**, réécrite avec `\r\x1b[2K`, et `render_output` ne l'émet que si `is_tty`.
Un log CI n'interprète pas ces séquences — il accumule des lignes — donc hors TTY il ne restait que les lignes permanentes, c'est-à-dire l'ouverture et la fermeture de la tâche.

## Décision

- Hors TTY seulement, `Task::increment` émet une **ligne permanente** (terminée par `\n`) portant le compteur, tous les ~5 % et à la complétion : ~21 lignes pour un scrape complet, environ une par minute au débit throttlé.
- Le pas est `(total / 20).max(1)` — un petit total (34 cours en `--subjects gex`) fait de chaque incrément un jalon, ce qui reste lisible.
- La complétion est un jalon explicite : 8855 = 20 × 442 + 15, le dernier élément ne tombe pas sur une frontière de pas.
- La décision est prise par une fonction pure, `milestone_line(is_tty, progress)`, qui rend la ligne ou `None` — `is_tty` est un paramètre, pas une lecture d'état global : sous `cargo test`, `io::stdout().is_terminal()` répond **vrai** en local (la capture de cargo intercepte `print!` via un thread-local, elle ne redirige pas le fd 1) et faux en CI ; un test qui dépend du `STATE` global serait donc vert ou rouge selon l'environnement.
- En TTY, rien ne change : la condition est fausse, l'affichage reste la ligne unique réécrite.

## Alternatives rejetées

- **Émettre la ligne transiente telle quelle hors TTY** : `\r\x1b[2K` s'affiche littéralement dans les logs GitHub Actions, et une ligne par cours ferait 8855 lignes de bruit.
- **Cadence temporelle (une ligne toutes les N secondes)** : il faudrait une horloge dans `PrintState`, et la sortie ne serait plus déterministe — donc plus testable sans injection de temps. Le jalon en pourcentage donne le même service à coût nul.
- **Groupes `::group::` de GitHub Actions** : spécifique à un CI, alors que le besoin est « une sortie lisible quand stdout n'est pas un terminal », vrai aussi pour `| tee`, `nohup` ou un cron local.
- **Forcer le mode TTY par variable d'environnement** : déplace le problème sur la configuration du workflow et laisse le log rempli de séquences d'échappement.
