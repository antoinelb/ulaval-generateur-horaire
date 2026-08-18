// Service worker (AIR DEG-3/BLD-3): cache-first with background
// revalidation, so every read path — snapshot included — survives a lost
// network once it has been seen once. Hand-written and vendored: no
// dependency, no build step (~30 lines is the whole strategy).
// NOTE: a service worker's scope is the directory it is served from, so to
// control the page this file must sit beside the index — `make ui-build`
// copies it there. Under `dx serve` it is absent and nothing registers.
const CACHE = "gh-v1";

self.addEventListener("install", () => self.skipWaiting());

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET" || !request.url.startsWith(self.origin ?? location.origin)) {
    return;
  }
  event.respondWith(
    caches.open(CACHE).then(async (cache) => {
      const cached = await cache.match(request);
      if (cached) {
        // stale-while-revalidate: serve what we hold, refresh underneath
        fetch(request)
          .then((response) => {
            if (response.ok) {
              cache.put(request, response.clone());
            }
          })
          .catch(() => {});
        return cached;
      }
      const response = await fetch(request);
      if (response.ok) {
        cache.put(request, response.clone());
      }
      return response;
    }),
  );
});
