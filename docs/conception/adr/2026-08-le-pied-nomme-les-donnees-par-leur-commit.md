# Le pied nomme le code et les données par leur commit, pas par une empreinte

Date : 2026-08-24

## Contexte

Le pied affichait `build <sha> … empreinte <fnv1a-64 des octets bruts>` (BLD-4).
Antoine a posé la bonne question : ces deux valeurs ne servent que si elles permettent de *retrouver* l'information.

Vérification faite, ni l'une ni l'autre n'était résoluble.

- **Le build hash** était bien injecté par `ui-build`, mais n'apparaissait nulle part dans GitHub Actions : `ci.yml` n'avait ni `run-name:`, ni `echo`, ni `$GITHUB_STEP_SUMMARY`.
  Pire, sur le chemin de rafraîchissement des données, `scrape.yml` déclenche `gh workflow run ci.yml --ref main` : le run est attribué à **main** alors que le job `deploy` bâtit le **dernier tag `v*`**.
  Le SHA visible dans la liste des runs n'était donc pas celui du pied.
- **L'empreinte** n'était calculée que dans le navigateur (`data.rs`), jamais imprimée, jamais journalisée, jamais dans un artéfact, et aucun binaire ne savait la calculer.
  Cinq reconstructions du flux d'octets depuis les fichiers sur disque ont toutes échoué à retomber sur une valeur affichée.
  Les commits de données, eux, portent tous le même message (`"Refreshed the scraped snapshots"`, `scrape.yml`) et sont indiscernables entre eux.

Le défaut de fond est structurel : le site déploie **deux sources indépendantes** — le code d'un tag, les données de `main` — et une seule était nommée.
L'empreinte était censée nommer l'autre, mais elle dérivait du *contenu* alors que git possède déjà un identifiant de ce contenu.

Le rapport d'essai `docs/ux/rapport-etudiante-2026-08-13.md` (ligne 253) avait tranché côté étudiante : « l'empreinte hexadécimale, elle, ne me sert à rien ».

## Décisions

- **L'empreinte disparaît.** `Provenance.data_hash` et `hash_raw` sont supprimés.
  `fnv1a_64` reste : `present.rs` en tire l'identifiant copiable de chaque erreur (ERR-1).
- **Le pied nomme deux commits** : le code (`BUILD_HASH`, déjà injecté) et les données (`DATA_HASH`, nouveau), chacun **lié à sa page GitHub**.
  Une capture d'écran mène désormais au code exact et aux données exactes qui l'ont produite — ce que BLD-4 visait.
- **`DATA_HASH` est surchargeable par l'environnement.** Le repli du makefile est `git log -1 --format=%h -- data/` ; le job `deploy` le surcharge avec `git log -1 --format=%h origin/main -- data/`, car il bâtit le code d'un tag par-dessus les données de `main` : le repli lirait l'historique du tag et nommerait le mauvais commit.
- **Un build local n'a pas de commit à montrer.** `BUILD_HASH`/`DATA_HASH` absents donnent « dev », rendu en texte simple et non en lien : TRU-1, on n'annonce jamais plus que ce qu'on sait, et un lien mort annoncerait une résolution qui n'existe pas.
- **Correction adjacente, dans le même changement** : le job `deploy` cherchait encore des tags `v*` (`if:` et `git tag --list`) alors que `release.yml` et le déclencheur `on.push.tags` étaient déjà passés aux versions nues (`0.2.0`).
  Les deux chemins de déploiement étaient morts — le push de tag était sauté par la condition, le `workflow_dispatch` de `scrape.yml` sortait en erreur faute de tag `v*`.
  Sans cette correction, rien de ce qui précède n'aurait jamais été déployé.
- **Le job `deploy` écrit un résumé** (`$GITHUB_STEP_SUMMARY`) listant le tag déployé, le SHA du code et celui des données, pour que le run porte lui-même ce que le pied affiche.

## Alternatives rejetées

- **Garder l'empreinte et la calculer aussi en CI** : imposait de remonter `hash_raw`/`fnv1a_64` dans `core`, de l'exposer par une sous-commande du scraper, et de tenir la couverture à 100 % sur trois crates — beaucoup de machinerie pour reconstruire un identifiant que git fournit déjà.
- **Se contenter d'un `run-name:` et d'un résumé de job**, en laissant l'empreinte dans le pied : rendait le *run* trouvable, mais laissait l'empreinte elle-même irrésoluble et le pied encombré d'une valeur inutile à tout le monde.
- **Afficher les deux (empreinte + commit)** : deux identifiants pour une même chose, dont un que personne ne sait résoudre. L'empreinte détectait en revanche un cas que le commit rate : des données servies depuis un cache de service worker périmé, où les octets ne correspondent plus au commit annoncé. Ce cas est réel mais rare, et la date de récolte à côté le signale déjà.
