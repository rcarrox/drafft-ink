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
            // Verify after fetching so diagnostics can distinguish offline/network,
            // HTTP and stale/rewritten files (SRI reports all of these as Failed to fetch).
            // Never cache an incomplete or mixed release.
            for (const asset of ASSETS) {
                const url = new URL(asset.path, BASE).href;
                try {
                    const response = await fetch(url, {cache: 'reload'});
                    if (!response.ok) throw new Error(`HTTP ${response.status} ${response.statusText}`);
                    const bytes = await response.clone().arrayBuffer();
                    // FTP ASCII transfer can rewrite only HTML line endings.
                    // Hash canonical LF text while keeping the original response
                    // in Cache Storage for the browser to display normally.
                    let integrityBytes = bytes;
                    if (asset.path.endsWith('.html')) {
                        const raw = new Uint8Array(bytes);
                        const normalized = new Uint8Array(raw.length);
                        let output = 0;
                        for (let input = 0; input < raw.length; input++) {
                            if (raw[input] === 13 && raw[input + 1] === 10) continue;
                            normalized[output++] = raw[input];
                        }
                        integrityBytes = normalized.subarray(0, output);
                    }
                    const digest = await crypto.subtle.digest('SHA-256', integrityBytes);
                    const actual = 'sha256-' + btoa(String.fromCharCode(...new Uint8Array(digest)));
                    if (actual !== asset.integrity) throw new Error('empreinte différente (fichier ancien, modifié ou réécrit par le serveur)');
                    let cached = response;
                    if (asset.path.endsWith('.mjs') && !/javascript|ecmascript/i.test(response.headers.get('Content-Type') || '')) {
                        const headers = new Headers(response.headers);
                        headers.set('Content-Type', 'text/javascript; charset=utf-8');
                        cached = new Response(bytes, {status: response.status, statusText: response.statusText, headers});
                    }
                    await cache.put(url, cached);
                } catch (error) {
                    const detail = error instanceof TypeError
                        ? 'échec réseau (fichier absent, chemin incorrect ou déploiement FTP incomplet)'
                        : error.message;
                    throw new Error(`${asset.path} : ${detail}`);
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
