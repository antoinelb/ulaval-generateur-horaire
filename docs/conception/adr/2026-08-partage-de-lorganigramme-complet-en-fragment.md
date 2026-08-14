# Partage de l'organigramme complet en fragment d'URL

Date : 2026-08-13

## Contexte

Le partage livré au jalon « partage URL » encodait **une session** (`?h=a2026.CODE:NRC…`) ; coller le lien exigeait en plus que l'horizon du destinataire contienne la session (« hors de votre horizon — ajustez le début »).
Note 9 d'Antoine (2026-08-13) : un lien doit porter **tout l'organigramme** et le destinataire ne doit rien faire ; le bouton appartient à l'en-tête, pas à la grille.

## Décision

Pipeline stateless (aucun serveur — `docs/conception/shareable_link.md`) :

`état → postcard → deflate brut (gardé seulement s'il rétrécit) → base64url → fragment #…`

- **`ShareV1` est gelée** : postcard encode par position, tout réordonnancement casserait silencieusement chaque lien déjà partagé. Un futur format sera une `ShareV2` avec son propre octet de version et une migration — jamais une édition de `ShareV1`. Le test `the_frozen_v1_string_still_decodes_byte_for_byte` verrouille l'octet près.
- Un octet d'en-tête `version|flag` distingue les versions et le deflate ; la décompression est **bornée à 256 Ko** (un lien hostile ne doit pas être une bombe de décompression).
- Les **cours manuels voyagent entiers**, chacun comme son propre JSON à l'intérieur de la struct (auto-descriptif exprès : `core::Course` peut évoluer sans corrompre les vieux liens) ; à l'import ils rejoignent la liste locale **avant** le parse du catalogue et le démarrage du worker — la copie locale d'un code déjà connu prime.
- Le **fragment** (`#`) plutôt que la query : il n'atteint jamais un serveur (pas de logs), et le payload base64url n'a pas besoin d'échappement.
- Import au montage de l'app : le plan du destinataire est remplacé entier par la porte d'annulation (« Organigramme partagé importé — Annuler restaure le vôtre »), fragment retiré de la barre d'adresse ensuite.
- L'ancien codec de session et le bouton de la grille sont **supprimés** (aucun lien en circulation) ; « Partager » vit dans l'en-tête.

## Alternatives rejetées

- Entrepôt côté serveur (IDs courts) : le projet est statique sans backend, par contrainte de charge.
- JSON compressé : 5–10× plus gros que postcard avant compression ; les noms de champs n'apportent rien qu'un octet de version ne donne pas.
- `Plan` sérialisé directement : sa forme évolue à chaque jalon ; le gel exige une struct dédiée aux types primitifs.

## Mesure

Un plan chargé (programme, 3 sessions garnies, un cours manuel complet) tient en ~560 caractères — loin sous les ~2000 des vieux navigateurs.
