# « Partager » confirme la copie, ou dit qu'elle a échoué

Date : 2026-08-30

## Contexte

Camille et Élodie relèvent indépendamment (2026-08-29) que « Partager » ne répond pas :

> « Rien ne change visuellement à l'écran. En vérifiant l'URL de la page, elle contient bien un nouveau fragment `#…` encodant l'état — donc l'action a fonctionné — mais rien à l'écran ne me le dit. »

Le bouton poussait pourtant un `AlertBody::Success`. Deux causes se cumulent :

- l'avis affirmait « Lien copié » **avant de savoir** : `browser::clipboard_write` lançait la promesse et l'abandonnait (`let _ = …`), donc un navigateur refusant le presse-papiers produisait quand même un ✓. C'est un `catch {}` déguisé (ERR-2) — et le ✓ n'était pas ce que l'étudiante regardait au moment où il passait ;
- les deux autres boutons « Copier » du panneau (le gabarit de cheminement, la fiche d'un cours manuel) avaient exactement le même défaut.

## Décision

- `browser::clipboard_write` devient `async` et **rend un booléen** : la promesse est attendue, son rejet est l'information. `false` ne veut pas dire « échec total » — le lien est déjà dans la barre d'adresse, écrite avant l'appel.
- « Partager » pousse ensuite :
  - `AlertBody::Success(present::share_note(true))` quand la copie a eu lieu. C'est la priorité la plus basse — une confirmation sur laquelle il n'y a rien à faire — donc le seul cas où ALR-4 tolère l'effacement automatique après 5 s ;
  - `AlertBody::Note(present::share_note(false))` sinon, qui dit ce qui a été refusé, ce qui reste vrai (le lien est dans la barre d'adresse) et quoi faire (Ctrl+L puis Ctrl+C) — ERR-1. Cet avis-là **persiste** : c'est une consigne, pas une félicitation.
- Les deux autres « Copier » suivent la même règle, avec `present::copied_note(quoi, copié)` — une formulation invariante en genre (« Copié dans le presse-papiers : … »), pour qu'une seule phrase serve le gabarit comme la fiche.
- Toute la rédaction vit dans `present`, testée nativement (AP-5) ; la vue ne choisit que le corps de l'avis, comme l'export JSON le fait déjà avec `export::menu::download_note`.

## Alternatives rejetées

- **Garder le `let _ =` et le ✓ inconditionnel** : ERR-2 — rien n'échoue en silence — et un faux « copié ✓ » est pire que pas d'avis du tout, puisqu'il fait coller un presse-papiers périmé.
- **Un avis persistant même en cas de succès** : ALR-3, l'étudiante partage plusieurs fois de suite ; une pile de ✓ à rejeter à la main est un bruit qu'aucune action ne justifie.
- **Se contenter du fragment dans la barre d'adresse** : c'est l'état rapporté — l'action a bien eu lieu, l'écran n'en dit rien, et il faut inspecter l'URL pour le savoir.
- **Un `document.execCommand('copy')` de repli quand la promesse est rejetée** : un chemin obsolète de plus à maintenir pour un cas où la barre d'adresse porte déjà la réponse.
