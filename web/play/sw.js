// Keeps a copy of the game on the device so it starts without a connection,
// also from the home screen.
//
// The Docker build fills in BUILD (a hash of everything listed in FILES) and
// FILES. Each deploy therefore changes this script, which makes the browser
// install the new version next to the running one. The page offers to switch
// over (see index.html); otherwise it takes over once the game is closed.

const BUILD = "__BUILD_ID__";
const FILES = __PRECACHE__;
const CACHE = `plane-chaos-${BUILD}`;

self.addEventListener("install", (event) => {
  // Fetched through the HTTP cache: files the page just loaded only cost a
  // revalidation, not a second download.
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(FILES)));
});

self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    for (const name of await caches.keys()) {
      if (name.startsWith("plane-chaos-") && name !== CACHE) await caches.delete(name);
    }
    // Control the page that installed us, so its later requests (assets Bevy
    // loads on demand) are served from this same version.
    await self.clients.claim();
  })());
});

self.addEventListener("message", (event) => {
  if (event.data === "skip-waiting") self.skipWaiting();
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) return;

  event.respondWith((async () => {
    const cache = await caches.open(CACHE);
    // /play/index.html and /play/?anything are the same page.
    const url = new URL(request.url);
    const key = request.mode === "navigate" && url.pathname.startsWith("/play/") ? "./" : request;
    return (await cache.match(key, { ignoreSearch: true })) ?? fetch(request);
  })());
});
