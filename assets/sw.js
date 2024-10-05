var cacheName = "oklch-color-picker-pwa";

async function cacheFirst(request) {
  const cached = await caches.match(request);
  if (cached) {
    return cached;
  }

  try {
    const res = await fetch(request);
    if (res.ok) {
      const cache = await caches.open(cacheName);
      cache.put(request, res.clone());
    }
    return res;
  } catch (error) {
    return Response.error();
  }
}

const cachePath =
  /\/$|\/index\.html$|\/oklch-color-picker-(\w|\d)*\.js$|\/oklch-color-picker-(\w|\d)*_bg\.wasm$/;

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (url.pathname.match(cachePath)) {
    event.respondWith(cacheFirst(event.request));
  }
});
