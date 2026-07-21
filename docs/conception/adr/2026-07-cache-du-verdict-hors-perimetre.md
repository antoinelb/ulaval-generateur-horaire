# Le verdict « hors périmètre » est mis en cache sous l'empreinte de la règle

Date : 2026-07-20

Étend `2026-07-cache-de-cours-parses` et révise une conséquence de `2026-07-cycles-hors-perimetre-sans-erreur`.

## Contexte

`2026-07-cycles-hors-perimetre-sans-erreur` a rendu 20 pages hors périmètre — 16 « Études post-MDD » (`MDD-5xxx`) et 4 jalons de thèse de troisième cycle (`PSY-785x`) — sous la forme `Ok(None)` : page comprise, délibérément sans cours.
`2026-07-cache-de-cours-parses` ne met en cache qu'un `Course` (`{"course": …, "years": …}`).
Une page qui n'en produit aucun n'écrit donc rien, et l'ADR concluait : « Ces pages ne sont pas mises en cache — rien à cacher — donc elles sont refetchées à chaque run. Vingt requêtes sur ~10 000 : négligeable. »

Négligeable en volume, mais visible : chaque relance, même intégralement servie par le cache, faisait **20 requêtes réseau** et n'était jamais hermétique.
Le bilan du run l'affichait honnêtement (`8806 cached, 20 fetched`) — c'est ce compteur, ajouté le même jour, qui a rendu le résidu observable.

Le raisonnement « rien à cacher » confondait deux choses : il n'y a pas de `Course` à sérialiser, mais il y a bien un **verdict** — « cette page ne donne aucun cours » — et ce verdict est un résultat de parsing aussi propre que l'est un cours.

Le piège à éviter est celui qui venait de coûter un scrape complet le matin même : un changement de format avait rendu tout le cache illisible **en silence**, et `read_cache` traitait chaque défaut comme un simple miss.
Mettre en cache un verdict « hors périmètre » rouvre exactement ce risque sous une autre forme : si la règle de périmètre change — un jour où `core::Cycle` gagnerait un troisième cycle — les sentinelles écrites sous l'ancienne règle mentiraient, et les pages resteraient exclues à tort.

## Décision

**Le verdict est mis en cache, stampé de l'empreinte de la règle de périmètre qui l'a produit, et retesté à chaque lecture.**

Un fichier de cache prend désormais l'une de deux formes disjointes, lues *untagged* :

```rust
#[serde(untagged)]
enum CacheEntry {
    Course(CachedCourse),                 // {"course": …, "years": …}
    OutOfScope { out_of_scope: String },  // {"out_of_scope": "1,2"}
}
```

`untagged` est sûr ici pour la même raison que `Credits` (`2026-07-credits-variables-en-enum`) : les deux variantes portent des champs requis disjoints (`course` contre `out_of_scope`), qu'aucune ne peut absorber.
Un fichier ne correspondant à ni l'une ni l'autre est un miss — un cache corrompu se refetch au lieu de mentir, ce qui est la propriété défensive déjà posée par `2026-07-cache-de-cours-parses`.
**Les fichiers positifs existants ne changent pas de forme** : `{"course": …}` se relit en `Course`, aucune relance froide n'est provoquée par ce changement.

L'empreinte est un *fingerprint* de la règle telle qu'elle est dans le code — les niveaux de cycle que `Cycle::try_from` accepte — et non une énumération de la réalité :

```rust
fn scope_tag() -> String {
    (0u8..=u8::MAX)
        .filter(|&level| Cycle::try_from(level).is_ok())
        .map(|level| level.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
```

Aujourd'hui elle vaut `"1,2"`.
Le balayage est borné sur `u8` (256 itérations), sans récursion.

À la lecture, une sentinelle n'est honorée **que si son empreinte égale encore celle d'aujourd'hui** :

- empreinte identique → hors périmètre, requête sautée (`Origin::Cache`);
- empreinte différente → la règle a changé, la sentinelle tombe comme un miss et la page est refetchée.

C'est le principe « cacher les entrées, recalculer les décisions » : la donnée stockée est l'empreinte (une entrée stable), la décision (sauter ou refetch) est recalculée contre la règle vivante à chaque lecture.
Un cache qui redérive sa propre validité ne peut pas diverger en silence du code — la garantie qui manquait précisément au format périmé du matin.

## Conséquences

Une relance intégralement en cache fait maintenant **0 requête** (`8826 cached, 0 fetched`) et retombe sous la seconde, au lieu des 20 requêtes précédentes.

**Aucune purge manuelle n'est requise quand la règle de périmètre change** : l'auto-invalidation par empreinte la rend inutile.
C'est un cran de mieux que ce que la règle générale de `2026-07-cache-de-cours-parses` promettait (`rm` du répertoire après un changement) — l'invalidation devient automatique pour ce cas précis.

La limite subsiste pour un autre changement : une correction de `cycle_level` (la lecture du *libellé* de cycle, pas la règle d'inclusion) n'est pas couverte par l'empreinte, et une sentinelle écrite avant cette correction survivrait.
C'est le même risque résiduel que pour un `Course` positif mis en cache avec un bogue propre, explicitement accepté par `2026-07-cache-de-cours-parses` : la parade reste le `rm` du répertoire, réservé à ce cas rare.

Un échec d'écriture de sentinelle est une anomalie `CourseError::Cache`, exactement comme pour un cours — le snapshot produit reste correct, et un test dédié couvre ce chemin.

## Alternatives rejetées

- **Ne rien cacher** (l'état d'avant) : 20 requêtes par run, une relance jamais hermétique, et un compteur `fetched` qui ne tombe jamais à zéro alors que rien de nouveau n'est lu.
- **Une sentinelle booléenne sans empreinte** (`{"out_of_scope": true}`) : plus courte, mais réintroduit exactement la péremption silencieuse du format périmé — un changement de règle laisserait les pages exclues à tort, et exigerait un `rm` manuel qu'on oublierait. Le fingerprint coûte quelques lignes et supprime le mode de défaillance.
- **Stamper le niveau de cycle observé et le retester** (`{"out_of_scope_level": 4}`, ré-appliquer `Cycle::try_from(4)` à la lecture) : auto-invalide aussi, et plus finement. Mais il faut que `parse` remonte le niveau sur le chemin `Ok(None)`, ce qui change sa signature et une douzaine de sites d'appel, alors que l'empreinte de la règle donne la même auto-invalidation sans toucher au parseur.
- **Cacher le HTML brut** : déjà rejeté par `2026-07-cache-de-cours-parses` pour son volume; un verdict de 22 octets règle le même problème pour ces pages.
