# Un run `--subjects` fusionne dans le snapshot au lieu de le remplacer

Date : 2026-07-20

## Contexte

`write_courses` écrivait chaque fichier de session sans condition, quel que soit le périmètre du run :

```rust
let stale = if subjects.is_empty() {
    stale_sessions(&sessions_dir, &sessions)
} else {
    Vec::new()
};
...
for (session, snapshot) in sessions {
    write_atomic(&path, &(json + "\n"))?;   // remplacement total, contenu filtré
}
```

Le commentaire juste au-dessus énonçait pourtant la bonne intention :

> only a full run has seen the whole catalogue, so only a full run may remove what it did not produce: a `--subjects` run knows nothing of the other subjects' sessions and must leave their files alone.

Seule la moitié en était implémentée.
La *suppression* était bien réservée au run complet, mais l'*écriture* ne l'était pas : un run filtré produisait un `BTreeMap` ne contenant que ses matières, et le réécrivait par-dessus le fichier complet.

Le dégât a été constaté sur le dépôt, causé par un `--subjects gex` lancé pour valider le parseur corrigé :

```
data/cours/a2026.json : 4151 cours → 15 (GEX seulement)
git diff --stat data/cours : 8 files changed, 464 insertions(+), 359171 deletions(-)
```

Récupérable ici parce que `data/cours` est commité (ADR `2026-07-catalogue-artefact-commite` vaut le même argument pour les snapshots).
Sur une machine de CI, ou entre deux commits, la perte serait définitive.

C'est exactement le mode de défaillance que la contrainte « ne jamais rien perdre en silence » vise : le run se termine avec succès, l'écriture est atomique, et le fichier obtenu est un JSON parfaitement valide — simplement amputé de 4136 cours.

## Décision

**Un run `--subjects` réécrit exactement les cours de ses matières dans chaque snapshot, et rien d'autre.**

Pour un run filtré, avant écriture :

1. lire chaque `{session}.json` **déjà présent**;
2. en retirer les cours dont la matière est dans le filtre;
3. y ajouter les cours produits par le run pour cette session;
4. trier par code, puis écrire.

Le balayage porte sur **toutes** les sessions présentes, pas seulement celles que le run produit : une matière qui quitte une session doit disparaître du fichier où elle siégeait, sans quoi le snapshot annoncerait une offre qui n'existe plus — le même raisonnement que celui qui justifie `stale_sessions` pour un run complet.

Le tri par code n'est pas cosmétique.
C'est l'ordre qu'un run complet produit (`course::group_by_session`, « these snapshots are committed, so they are sorted by code … to keep the git diffs meaningful »).
Un run par matière doit rendre un fichier **identique à ce qu'aurait produit un run complet**, sinon le diff cesse d'être lisible et la propriété que le tri achetait est perdue.

**Un snapshot existant illisible arrête le run.**
Poursuivre la fusion en ignorant le fichier reviendrait à écraser toutes les matières qu'il contenait — précisément la perte que la fusion existe pour empêcher.

Un run complet (`subjects.is_empty()`) garde le chemin d'avant : remplacement intégral, plus suppression des sessions périmées.
Il a vu tout le catalogue, il fait donc autorité sur l'ensemble du dossier.

## Conséquences

`SessionSnapshot` gagne `Deserialize` : le scraper relit désormais sa propre sortie.

La règle « `{session}.manuel.json` n'est jamais touché » (ADR `2026-07-contribution-de-cours-manuels`) était dupliquée dans le filtre de `stale_sessions`; elle est maintenant portée par un seul helper, `session_files`, partagé par la fusion et le balayage des périmés.
Une règle de sécurité écrite à deux endroits finit par diverger.

Le cas de test qui plantait `h2024.json` avec la chaîne `"other subjects"` est renforcé : il plante un vrai snapshot, et vérifie que le cours qu'il contient survit — l'ancienne version ne pouvait pas voir la perte, puisqu'elle ne regardait que l'existence du fichier.

Le cas de test principal, `a_scoped_run_rewrites_its_own_subject_inside_a_shared_snapshot`, a été vérifié rouge avant correction, avec exactement la signature de l'incident :

```
  left: ["GEX-1000"]
 right: ["GCI-1000", "GEX-1000", "GZZ-1000"]
```

Les deux matières encadrant GEX alphabétiquement sont choisies exprès : une fusion qui ajouterait à la fin au lieu de trier passerait le test de préservation mais échouerait celui-ci.

## Alternatives rejetées

- **Écrire les runs filtrés dans un dossier séparé** : aucune fusion à écrire, aucun risque d'écrasement. Mais `--subjects` sert justement à itérer vite sur les vrais snapshots pendant une correction du parseur; produire une sortie qu'il faut ensuite recopier à la main réintroduit le risque, déplacé.
- **Refuser d'écrire les fichiers de session sur un run filtré** (n'écrire que le journal) : sûr, et rend l'option inutilisable pour son seul usage réel.
- **Fusionner sans trier, en ajoutant à la suite** : plus court, et casse la propriété que `group_by_session` paie déjà — un diff de run par matière deviendrait illisible, alors que ces fichiers sont commités et relus.
- **Ne lire que les sessions que le run produit** : suffit pour préserver les autres matières, mais laisse pour toujours les cours d'une matière dans une session qu'elle a quittée. C'est la moitié de bug que ce même code avait déjà pour la suppression.
- **Ignorer un snapshot illisible et écrire quand même** : le run ne s'arrêterait pas, au prix exact de la perte qu'on corrige.
