# L'écart entre « Tout geler » et « Réinitialiser » tombe au rythme commun

Date : 2026-08-30

## Contexte

Dans la barre du haut, « Tout geler » et « Réinitialiser » étaient séparés de 32 px : le `gap: 1rem` de `.header-bar`, plus le `margin-left: 1rem` de `.header-reset`.
Dans la bande de statut, « Partager » et « Exporter ▾ » le sont de 8 px : le `gap: 0.5rem` de `.status-exports`, leur emballage commun.

Antoine demande le même écart aux deux endroits.

L'écart de la barre du haut n'est pas un reste de mise en page : il vient de l'ADR `2026-08-barre-du-haut-degarnie` et sert ACT-5 — un contrôle destructeur ne jouxte pas un contrôle fréquent.
Il valait 3 rem quand « Partager », geste courant, bordait « Réinitialiser », puis 2 rem quand « Tout geler » a pris cette place (ADR `2026-08-tout-geler-sur-une-ligne-et-verdict-ecourte`).

## Décision

**Les deux boutons sont emballés dans `div.header-actions`**, exactement comme `.status-exports` emballe « Partager » et « Exporter ». Le conteneur porte `display: flex; gap: 0.5rem; flex: none;` et devient le seul porteur de l'écart ; le `margin-left` de `.header-reset` disparaît, la règle gardant ses deux autres déclarations (`color`, `border-color`), qui portent la teinte d'accent.

`flex: none` sur le groupe reprend pour lui ce que `.header-bar .status-undo` donne aux boutons : c'est le groupe, maintenant, qui est l'élément flexible de la barre, donc lui que la barre serrée comprimerait, et le repli de « Tout dégeler » sur deux lignes reviendrait.
La règle `.header-bar .status-undo { flex: none; }` continue de s'appliquer aux deux boutons : son combinateur est une espace, donc un descendant à n'importe quelle profondeur, et non un enfant direct.

## Ce que l'écart ne protège plus

**ACT-5 n'est plus satisfait par la séparation spatiale.** À 8 px, l'écart est celui de tous les autres voisinages de l'interface ; il ne dit plus rien de particulier, et il faut le dire plutôt que prétendre le contraire.

Ce qui tient le geste à sa place :

- **Le voisin est inoffensif.** « Tout geler » est un geste rare et entièrement annulable, annulable par lui-même puisque le second clic défait le premier. Un clic de travers dans ce sens ne coûte rien — ce qui n'était pas vrai de « Partager », le voisinage que les 3 rem d'origine visaient.
- **Le libellé et la teinte distinguent le bouton.** « Réinitialiser » garde la teinte d'accent que `.header-reset` est seul à porter, et la couleur ne porte pas seule la différence : le mot le fait (INP-3).
- **Le retour existe.** « Réinitialiser » est annulable, et son propre avis porte « Annuler » (ADR `2026-08-reinitialiser-annulable-depuis-son-avis`) ; ACT-2 est servi, jamais par une boîte de confirmation.
- **Rien ne se cache.** Le bouton reste à découvert, jamais dans un menu de débordement (LAY-7).

C'est la décision d'Antoine, prise en connaissance de la règle. La sécurité du geste repose désormais sur ces quatre points seuls, sans l'écart.

## Alternatives rejetées

- **`margin-left: 0` sur `.header-reset`** : insuffisant. Il resterait le `gap: 1rem` de `.header-bar`, soit 16 px — le double de la cible.
- **`margin-left` négatif** (`-0.5rem`) : atteint la valeur, mais par soustraction d'un écart qu'on vient d'ajouter. Une rustine qui se périme au premier changement du `gap` de la barre.
- **Baisser le `gap` de `.header-bar` à 0.5 rem** : toucherait tous les voisinages de la barre — logo, titre, sous-titre, « changer », avertissement, compte de crédits — pour régler celui de deux boutons.
- **Garder 2 rem et refuser la demande** : ACT-5 ne s'applique plus à ce voisinage comme il s'appliquait à « Partager ». La règle demande une séparation face à un contrôle *fréquent* ; le voisin ne l'est pas, et l'écart n'est plus le seul dispositif de sécurité du bouton.
