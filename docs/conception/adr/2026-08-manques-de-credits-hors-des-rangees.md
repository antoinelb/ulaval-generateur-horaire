# Les manques de crédits ne s'affichent plus par rangée de cours

Date : 2026-08-27

## Contexte

L'audit LAY-2 (V5) a montré que `RowView` injectait, sous chaque cours concerné, un `div.panel-course-sub--error` par manque de crédits (`crate::solve::course_shortfall_messages`), rendu quand le worker répondait — un ajout asynchrone qui décale tout ce qui suit la rangée.
Comparaison des deux fonctions dans `crates/ui/src/solve.rs` : `course_shortfall_messages(code, shortfalls, plan)` filtre `shortfalls` sur `code` puis appelle `credit_shortfall_message` sur chacun — la même fonction que celle déjà utilisée pour `shortfall_messages` dans la zone de verdicts réservée (`panel.rs`, `OrganigrammeControls`).
Le texte produit est donc strictement identique aux deux endroits, code du cours inclus (`credit_shortfall_message` commence son message par `shortfall.code`) : la rangée n'apportait aucune information que la zone de verdicts ne portait déjà.

## Décision

L'injection par rangée est retirée : `RowView` ne lit plus `credit_shortfalls` ni le contexte `solver`, et la classe CSS `.panel-course-sub--error` (devenue inutilisée) est supprimée.
L'information — quel cours, quelle session, combien de crédits manquent — continue de s'afficher, une seule fois, dans `div.panel-verdicts` (ADR `2026-08-verdicts-du-panneau-sans-hauteur-reservee`), qui nomme déjà le cours en toutes lettres.
Plus rien d'asynchrone ne s'insère dans les rangées de cours.

## Alternatives rejetées

- **Garder la rangée et retirer le message global** — la zone de verdicts est le seul endroit qui reste stable pendant une résolution (V2) ; le message par rangée est justement celui qui décale la rangée suivante à chaque réponse du worker.
- **Différencier les deux messages pour justifier de garder les deux** — les textes sont produits par le même appel à `credit_shortfall_message` ; il n'existe aucune information supplémentaire à faire porter par la rangée.
