var cacheName = "oklch-color-picker-pwa";

async function networkFirst(request) {
  try {
    const networkResponse = await fetch(request);
    if (networkResponse.ok) {
      const cache = await caches.open(cacheName);
      cache.put(request, networkResponse.clone());
    }
    return networkResponse;
  } catch (error) {
    const cachedResponse = await caches.match(request);
    return cachedResponse || Response.error();
  }
}

const cachePath =
  /\/$|\/index\.html$|\/oklch-color-picker-(\w|\d)*\.js$|\/oklch-color-picker-(\w|\d)*_bg\.wasm$/;

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (url.pathname.match(cachePath)) {
    event.respondWith(networkFirst(event.request));
  }
});
