const cachePrefix = "oklch-color-picker-";
const cacheName = "oklch-color-picker-development";
const precachedResources = [];

async function precache() {
  const cache = await caches.open(cacheName);
  await cache.addAll(precachedResources);
  await self.skipWaiting();
}

async function activate() {
  const cacheNames = await caches.keys();
  const obsoleteCacheNames = cacheNames.filter(
    (name) => name.startsWith(cachePrefix) && name !== cacheName,
  );
  await Promise.all(obsoleteCacheNames.map((name) => caches.delete(name)));
  await self.clients.claim();
}

async function loadNavigation(request) {
  try {
    return await fetch(request);
  } catch {
    const cache = await caches.open(cacheName);
    return (await cache.match("index.html")) ?? Response.error();
  }
}

async function loadAsset(request) {
  const cache = await caches.open(cacheName);
  return (await cache.match(request)) ?? fetch(request);
}

self.addEventListener("install", (event) => {
  event.waitUntil(precache());
});

self.addEventListener("activate", (event) => {
  event.waitUntil(activate());
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin) {
    return;
  }

  event.respondWith(
    request.mode === "navigate" ? loadNavigation(request) : loadAsset(request),
  );
});
