/* Generated into sw.js by utils/build_offline_cache.py after the WASM build. */
const BUILD = __Q_CACHE_BUILD__;
const ASSETS = __Q_CACHE_ASSETS__;
const BASE = new URL('./', self.location.href);
const PREFIX = `qraphtinc-offline:${BASE.pathname}:`;
const CACHE = PREFIX + BUILD;
const FILES = new Map(ASSETS.map(asset => [new URL(asset.path, BASE).href, asset]));

self.addEventListener('install', event => {
    event.waitUntil((async () => {
        const cache = await caches.open(CACHE);
        try {
            // Integrity rejects a partially uploaded release or a host serving
            // its home page instead of the requested JS/WASM. Never cache data APIs.
            for (const asset of ASSETS) {
                const url = new URL(asset.path, BASE).href;
                try {
                    const response = await fetch(url, {cache: 'reload', integrity: asset.integrity});
                    if (!response.ok) throw new Error(`HTTP ${response.status}`);
                    await cache.put(url, response);
                } catch (error) {
                    throw new Error(`${asset.path} : ${error.message}`);
                }
            }
        } catch (error) {
            await caches.delete(CACHE);
            for (const client of await self.clients.matchAll({includeUncontrolled: true})) {
                client.postMessage({type: 'Q_CACHE_ERROR', detail: error.message});
            }
            throw error;
        }
        // Updates wait for explicit user activation; the first install activates normally.
    })());
});

self.addEventListener('activate', event => {
    event.waitUntil((async () => {
        for (const key of await caches.keys()) {
            if (key.startsWith(PREFIX) && key !== CACHE) await caches.delete(key);
        }
        await self.clients.claim();
    })());
});

self.addEventListener('message', event => {
    if (event.data?.type === 'Q_ACTIVATE_UPDATE') self.skipWaiting();
});

self.addEventListener('fetch', event => {
    if (event.request.method !== 'GET') return;
    const url = new URL(event.request.url);
    if (url.origin !== BASE.origin) return;
    url.search = '';
    const index = new URL('index.html', BASE).href;
    const key = event.request.mode === 'navigate' &&
        (url.href === BASE.href || url.href === index) ? index : url.href;
    if (!FILES.has(key)) return;
    event.respondWith((async () => {
        const cache = await caches.open(CACHE);
        return await cache.match(key) || fetch(event.request);
    })());
});
