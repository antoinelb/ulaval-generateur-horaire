# Cours obligatoires d'une concentration

Date : 2026-07-20

## Contexte

`Concentration` était `{title, credits_required, rules}`.
Le bac en génie industriel (4 concentrations sur 5) et le bac en génie mécanique (2 sur 3) placent un accordéon « Cours obligatoires » **à l'intérieur** d'une concentration — par exemple `GMC-3351` pour « Robotique », `GIN-4021` pour « Ingénierie de la chaîne logistique ».
Le type ne pouvait pas les représenter.

Le même problème se pose sur un bloc du programme : la maîtrise en génie des eaux ajoute un bloc « Recherche » dont le seul accordéon est « Cours obligatoires » (GEX-6811 à 6814).

## Décision

1. `Concentration` gagne `mandatory: Vec<String>`, miroir exact de `Profile`.
   Les trois rôles d'un bloc — programme, concentration, profil — sont dès lors lus par la même fonction : un accordéon « Cours obligatoires » alimente `mandatory`, tout autre accordéon devient une `Rule`.
2. Un bloc du programme autre que le premier verse ses « Cours obligatoires » dans le `mandatory` du programme.
   La maîtrise passe ainsi de 2 à 6 cours obligatoires, et la règle « Recherche » de l'ancienne fixture disparaît : ses quatre activités valent 7 + 7 + 7 + 9 = 30 crédits, le mémoire que la page décrit elle-même, et 15 + 30 = 45 = les crédits exigés.
3. `Concentration.credits_required` passe de `i64` à `Option<i64>`, comme `Profile`.
   Les six pages connues portent toutes « N crédits exigés », mais l'`Option` retire au parseur une branche d'échec sans rien changer à la sérialisation (`skip_serializing_if`).

## Alternatives rejetées

- **Une règle synthétique `{"count": N}`** avec N = longueur de la liste : fabrique une contrainte que la page n'écrit nulle part, et « choisir N parmi N » ment sur l'intention — ce sont des cours imposés, pas un choix.
- **Une règle hors grammaire (`raw` seul)** : ne perd rien, mais rend la liste de cours inexploitable par le solveur alors qu'elle est parfaitement structurée.
- **Déduire les crédits du bloc « Recherche » par soustraction** (45 − 15 = 30), ce que faisait l'ancienne fixture : la valeur n'existe nulle part dans le HTML, et une fixture qu'aucun parseur ne peut reproduire n'est pas un cas de test.
