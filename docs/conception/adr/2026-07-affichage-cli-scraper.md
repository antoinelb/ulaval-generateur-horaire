# Affichage CLI du scraper : ligne transiente unique, écrite à la main

Date : 2026-07-13

## Contexte

Le binaire `scraper` doit montrer sa progression (~10 000 requêtes, ~20 min) à deux publics : un humain dans un terminal et le log du cron CI.
Le style visé vient d'un projet Python antérieur (holmes) : ligne de chargement `[✱]` réécrite sur place, ligne de succès `[+]`, compteur `i/n`, plus deux besoins nouveaux — des blocs indentés ouverts/fermés par des tâches, et une ligne de progression alimentée par des workers concurrents.
L'étude de deux implémentations existantes (uv : `indicatif::MultiProgress`, redessin complet d'une région multi-lignes ; pacman : suivi manuel du curseur avec déplacements relatifs) a montré que le coût vient des lignes vivantes multiples ; une seule ligne transiente en bas d'écran ne demande aucune arithmétique de curseur.

## Décision

Un module `print.rs` dans la crate `scraper`, sans dépendance (codes ANSI et `std` seulement).

- **Une seule ligne transiente**, toujours en bas ; tout le reste est du scrollback permanent.
  La réécrire ne coûte que `\r` + effacement de ligne (`\x1b[2K`).
- **Tout l'état mutable derrière un seul `Mutex<PrintState>`** : indentation, en-tête en attente (`pending`), progression active (`progress`).
  La ligne du bas est une **fonction pure de cet état** (en-tête avec ou sans compteur, sinon ligne de statut, sinon rien) ; aucune donnée d'affichage n'est stockée ailleurs.
- **Tâches adaptatives via gardes RAII** : `task()` affiche un en-tête transient `[✱]` ; si rien ne s'imprime dans le bloc, `done(self)` le réécrit sur place en `[+]` (une seule ligne) ; sinon le premier print matérialise l'en-tête en `[→]` permanent et `done` ferme le bloc à l'indentation du contenu.
  Un `Drop` sans `done()` imprime `[x]` : le chemin d'erreur est le défaut, le succès se réclame explicitement.
- **Progression concurrente** : `task_progress(msg, done_msg, total)` + `inc(&self)` ; les workers n'impriment jamais, ils incrémentent sous le verrou (compteur strictement monotone à l'écran).
  `inc` est la seule opération qui ne matérialise pas l'en-tête en attente.
  Une seule tâche comptée à la fois ; une deuxième garde sa structure mais son compteur est ignoré et signalé par `[!]`.
- **Dégradation hors terminal** (`stdout().is_terminal()`) : même machine à états, mais rien de transient n'est dessiné ; le log CI montre la même structure de blocs, sans animation.
- **L'affichage n'est pas critique** : toute erreur d'écriture est avalée (`.ok()`) ; la sortie réelle du scraper est les fichiers sur disque.

Plafonds assumés (documentés en commentaires dans le code) : réécrire une ligne au-dessus du bas (p. ex. re-transformer `[→]` en `[+]` à la fermeture) ou plusieurs lignes vivantes exigeraient le suivi de curseur à la pacman (~30 lignes) ; les blocs appartiennent au fil orchestrateur, seuls les `increment` viennent des workers.

## Alternatives rejetées

- **`indicatif` (+ `console`)** : résout les régions multi-lignes dont on n'a pas besoin à 10 req/s ; une dépendance lourde pour une seule ligne épinglée.
- **Région multi-lignes à la pacman** (une ligne par worker, curseur relatif) : élégant mais superflu — le débit est borné par le throttle réseau, une ligne `i/n` porte toute l'information ; conservé comme voie d'évolution documentée.
- **Spans `tracing` + subscriber indenté** : même modèle conceptuel (garde = portée), mais machinerie de logging structuré injustifiée pour un binaire CLI ; la migration resterait naturelle si le besoin apparaît.
- **Compteurs atomiques épars** (`AtomicUsize` pour l'indentation et la progression) : plus léger en apparence, mais l'affichage `i/n` perd sa monotonie (un `3/10` peut écraser un `4/10`) et les invariants « ces champs changent ensemble » ne sont plus portés par le type ; le `Mutex` unique les encode.
