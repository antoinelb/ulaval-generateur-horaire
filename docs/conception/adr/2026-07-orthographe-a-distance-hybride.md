# « À distance-hybride » est le mode hybride

Date : 2026-07-20

## Contexte

L'ADR `2026-07-mode-hybride` a ajouté `Mode::Hybrid` pour le libellé « Hybride » rencontré sur GEX et GCI.
Le premier scrape de GMC a produit une anomalie de plus sur le même en-tête `span.header--content-details` :

```
Malformed entry for mode: À distance-hybride
```

— GMC-7000, section DH1, Automne 2020.
`parse_mode` ne connaissant pas ce libellé, la section entière était écartée, donc son horaire perdu (le mode se lit dans l'en-tête, jamais dans le « Type: » d'une plage).

Le site écrit ainsi la même chose que « Hybride » : une partie en ligne, une partie en classe.
Rien dans la page ne distingue les deux orthographes autrement que par le texte de l'en-tête.

## Décision

`parse_mode` accepte « À distance-hybride » comme « Hybride » et rend `Mode::Hybrid` — un seul bras de `match`, aucune variante nouvelle dans `core::Mode`.
La composition décrite par l'ADR `2026-07-mode-hybride` s'applique telle quelle : les plages sans « Journée: » ni « Horaire: » se retirent d'elles-mêmes dans `parse_slot`, donc une section hybride ne rend que ses rencontres en classe.
Toute autre orthographe reste une anomalie signalée.
GMC-7000 rejoint les fixtures gelées.

## Alternatives rejetées

- **Une variante `RemoteHybrid` distincte** : le domaine n'a pas deux modes ici, il a deux orthographes du même ; l'UI et le solveur devraient traiter un cas qui ne veut rien dire de plus.
- **Reconnaître par sous-chaîne (`contains("ybride")`)** : accepterait n'importe quel libellé futur sans qu'on le sache, alors que la table de libellés exacts est précisément ce qui rend une orthographe inconnue visible.

## Effet mesuré

Sur GMC, l'anomalie de mode a disparu et GMC-7000 revient dans le snapshot `a2020` avec sa section DH1 (`"mode": "hybrid"`, aucune plage horaire : sa seule plage est en ligne).
