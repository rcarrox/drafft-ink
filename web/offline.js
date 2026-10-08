/* Offline app shell only. Documents/fonts remain in their existing local stores. */
(() => {
    if (!('serviceWorker' in navigator) || !window.isSecureContext) return;
    // Existing canvas diagnostic tests deliberately bypass caching; dedicated offline tests exercise it.
    if (new URLSearchParams(location.search).has('drafftink-test') && !new URLSearchParams(location.search).has('offline-test')) return;
    let registration, updateRequested = false, started = false;
    let hadController = !!navigator.serviceWorker.controller;
    const notice = document.createElement('div');
    notice.id = 'qraphtinc-offline';
    notice.style.cssText = 'position:fixed;right:12px;bottom:12px;z-index:10000;font:12px system-ui;color:#222;background:#fff;border:1px solid #ddd;border-radius:8px;padding:7px 10px;box-shadow:0 2px 6px #0002;max-width:290px';
    notice.setAttribute('role', 'status');
    window.addEventListener('keydown', event => {
        if (event.ctrlKey && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'p') {
            notice.hidden = !notice.hidden;
        }
    });
    const show = text => { notice.textContent = text; if (!notice.isConnected) document.body.appendChild(notice); };
    const dismiss = () => {
        const button = document.createElement('button');
        button.textContent = 'Fermer';
        button.style.cssText = 'margin-left:8px;border:0;background:none;cursor:pointer';
        button.onclick = () => notice.remove();
        notice.appendChild(button);
    };
    const ready = () => {
        if (registration?.waiting) return offerUpdate();
        show(navigator.onLine ? 'Disponible hors connexion' : 'Mode hors connexion');
        dismiss();
        // No permanent overlay over canvas/panels once readiness was announced.
        setTimeout(() => { if (!registration?.waiting) notice.remove(); }, 6000);
    };
    const offerUpdate = () => {
        show('Nouvelle version disponible.');
        const button = document.createElement('button');
        button.textContent = 'Installer et recharger';
        button.style.cssText = 'margin:6px 0 0;padding:5px 8px;cursor:pointer';
        button.onclick = () => {
            if (!confirm('Enregistrez votre travail avant de recharger. Installer la nouvelle version ?')) return;
            updateRequested = true;
            registration.waiting?.postMessage({type: 'Q_ACTIVATE_UPDATE'});
        };
        notice.appendChild(document.createElement('br'));
        notice.appendChild(button);
        dismiss();
    };
    navigator.serviceWorker.addEventListener('controllerchange', () => {
        if (updateRequested) location.reload();
        else if (!hadController) {
            hadController = true;
            ready();
        }
        else if (started) {
            // Another tab may activate the update. Do not reload this sheet.
            show('Version actualisée. Rechargez après avoir enregistré votre travail.');
            dismiss();
        }
    });
    async function start() {
        if (started) return;
        started = true;
        try {
            show('Préparation du mode hors connexion…');
            registration = await navigator.serviceWorker.register('./sw.js', {scope: './', updateViaCache: 'none'});
            const watch = worker => worker?.addEventListener('statechange', () => {
                if (worker.state === 'installed') {
                    if (registration.waiting && navigator.serviceWorker.controller) offerUpdate();
                }
                if (worker.state === 'redundant') { show('Cache hors connexion indisponible. Réessayez après le chargement complet des fichiers.'); dismiss(); }
            });
            watch(registration.installing);
            registration.addEventListener('updatefound', () => watch(registration.installing));
            if (registration.waiting) offerUpdate();
            else navigator.serviceWorker.ready.then(ready);
            registration.update().catch(() => {});
        } catch (error) {
            console.warn('Offline cache unavailable:', error);
            show('Cache hors connexion indisponible sur cet hébergement.');
            dismiss();
        }
    }
    // Avoid a second WASM download while the application's first load is running.
    window.addEventListener('qraphtinc-ready', start, {once: true});
    if (document.querySelector('canvas') && !document.getElementById('loading')) start();
    window.addEventListener('online', () => { registration?.update().catch(() => {}); });
    window.addEventListener('offline', () => { if (navigator.serviceWorker.controller) ready(); });
})();
