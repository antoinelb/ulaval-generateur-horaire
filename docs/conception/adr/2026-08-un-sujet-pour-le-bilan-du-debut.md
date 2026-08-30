# Le bilan d'un changement de « Début » a un sujet

Date : 2026-08-30

## Contexte

Rapport directeur-gci du 2026-08-29 : « après deux ou trois changements de "Début", cinq à six avertissements quasi identiques s'empilent dans le coin, dont plusieurs redondants au mot près, sans jamais se nettoyer automatiquement. J'ai dû les fermer un par un. »

La cause est locale et entièrement mécanique. Le `onchange` du sélecteur « Début » poussait son bilan par `push_alert`, donc :

- **sans sujet** — or `AlertStack::push` ne déduplique que sur le corps *exact*, et le libellé change à chaque fois puisqu'il énumère les sigles délogés. Deux changements successifs produisent deux textes différents, donc deux avis ;
- **avec la cause `Sticky`**, qui ne se périme jamais.

Le mécanisme correct existait déjà et avait réglé exactement le même empilement pour les changements de concentration : `push_topic_alert` + un `AlertTopic` (`ScopeDepartures`). Il n'a simplement pas été appliqué ici.

## Décision

`AlertTopic::StartMove`. Le bilan passe par `push_topic_alert`, avec la cause `Document`.

- **Un sujet** : trois changements de Début laissent **un** avis, celui du dernier. Le rejet mémorisé par libellé exact (`2026-08-toasts-un-par-sujet-et-rejet-memorise`) s'applique dès lors normalement.
- **`Document` et non `Sticky`** : c'est le bilan d'un acte passé, vrai tant qu'on travaille sur ce document, faux dès qu'on en ouvre un autre — un avis qui nomme des sigles délogés d'un cheminement n'a rien à dire sur le suivant.

Le libellé n'est pas retouché : il énumère toujours les sigles avant de dire ce qui leur arrive. C'est lisible dans un toast, qui affiche le texte entier ; ça ne le serait pas dans une rangée d'une ligne, mais cette forme-là n'est pas celle qu'on a (`2026-08-alertes-en-toasts-flottants`).

## Test de régression

`alerts::tests::three_start_moves_leave_one_alert` pousse les trois libellés que `present::start_move_note` produit réellement pour un, deux puis trois sigles, et vérifie qu'il n'en reste qu'un. Les trois textes diffèrent : c'est précisément ce qui rend la déduplication par corps impuissante et le sujet nécessaire.

Limite assumée : le test couvre le mécanisme, pas le site d'appel. Le `onchange` vit dans `crates/ui/src/components/`, exclu de la couverture et sans harnais de vue — un retour à `push_alert` à cet endroit ne ferait échouer aucun test.

## Alternatives rejetées

- **Dédupliquer sur un préfixe du corps.** Une heuristique de chaîne pour retrouver une identité qu'un sujet déclare exactement ; le premier libellé reformulé la casse en silence.
- **Cause `Sticky` avec un sujet.** Réglerait l'empilement mais garderait l'avis à l'écran après une bascule de document, où il décrit un cheminement qui n'est plus ouvert.
