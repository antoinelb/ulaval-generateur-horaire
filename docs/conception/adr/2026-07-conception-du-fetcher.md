# Fetcher : throttle partagé, Retry-After honoré, logique testable

Date : 2026-07-17

## Contexte

Le module `fetch.rs` sert deux phases : le catalogue (~205 pages) et les pages cours (~10 000 pages), toutes deux throttlées à ~10 req/s.
En séquentiel, le débit réel est borné à max(100 ms, latence par page) : la concurrence sert à **atteindre** le throttle, pas à le dépasser.
Deux exigences ont été posées à la conception : toute la logique doit être testable (unitaire ou intégration), et `Retry-After` doit être honoré — on est de bons citoyens du web ou on ne l'est pas.

## Décision

- **Async (tokio + reqwest), pas de threads** : travail purement IO-bound, runtime déjà dans la pile, orchestration réduite à `buffer_unordered(~4)` sur un `Fetcher` partagé.
- **Throttle = état partagé** : `Fetcher { client, next_allowed: tokio::sync::Mutex<Instant> }`, `fetch(&self, url)`.
  Le verrou est tenu à travers le `sleep_until` : les créneaux sont distribués un à un, espacés de `min_interval` (token bucket de taille 1).
  Un seul `Fetcher` est construit dans `main` et passé partout — structurellement une seule limite de débit.
- **`Retry-After` honoré** (RFC 9110 § 10.2.3) : formes delta-seconds et HTTP-date (`httpdate`, déjà dans l'arbre de dépendances via hyper).
  L'attente pousse le `next_allowed` **partagé** : tous les workers reculent ensemble, pas seulement celui qui a reçu le 429/503.
  Plafond de 5 min — au-delà, erreur qui arrête le run plutôt qu'une attente non bornée.
- **Retries bornés** : 3 tentatives par URL; on réessaie sur erreur de transport, 5xx et 429; tout autre 4xx est permanent.
  Le throttle est reconsulté à chaque tentative — l'espacement ne dépend pas d'une coïncidence entre backoff et intervalle.
- **Client** : timeout de 30 s (reqwest n'en a aucun par défaut — une connexion pendue bloquerait le cron), user agent honnête avec contact.
- **Testabilité** : les décisions sont des fonctions pures (`should_retry(status)`, `parse_retry_after(headers, now)`) testées unitairement.
  `wait_for_slot` se teste sur l'horloge virtuelle de tokio (`start_paused` — jamais mélangée à de l'IO réel).
  Le comportement HTTP complet (corps sur 200, retry sur 503 + `Retry-After`, 404 permanent, épuisement des retries) se teste contre `wiremock` (dev-dependency).
  `min_interval` et le backoff sont injectés par `new()`; les valeurs réelles vivent dans `main.rs`.

## Alternatives rejetées

- **Threads** : rien de CPU-bound; machinerie de canaux et de join en plus pour le même résultat.
- **ureq bloquant + sleep** : plus simple pour le catalogue seul, mais la phase cours a besoin de concurrence pour atteindre 10 req/s (latence probable > 100 ms), et deux piles HTTP n'en valent pas une.
- **`fetch(&mut self)` sans mutex** : la séquentialité garantie par le borrow checker était élégante, mais interdit le partage entre workers — refactor inévitable dès la phase cours.
- **Backoff exponentiel avec jitter** : surdimensionné pour un client déjà throttlé à 10 req/s; un backoff court fixe + `Retry-After` suffisent.
- **Ignorer `Retry-After`** : rejeté explicitement — politesse non négociable, et l'honorer via le `next_allowed` partagé simplifie le design au lieu de le compliquer (aucune machine à états de backoff par worker).
