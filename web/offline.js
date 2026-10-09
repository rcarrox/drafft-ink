/* Offline app shell only. Documents/fonts remain in their existing local stores. */
// PDF.js stays unloaded until the user explicitly imports a PDF.
let qursoPdfModule;
window.drafftinkPdfPages = async file => {
    window.drafftinkPdfPagesTruncated = false;
    qursoPdfModule ||= import('./pdfjs/pdf.min.js');
    const pdfjs = await qursoPdfModule;
    pdfjs.GlobalWorkerOptions.workerSrc = new URL('./pdfjs/pdf.worker.min.js', location.href).href;
    const pdf = await pdfjs.getDocument({data: new Uint8Array(await file.arrayBuffer()), useSystemFonts: true}).promise;
    try {
        const pages = [];
        const canvas = document.createElement('canvas');
        const count = Math.min(pdf.numPages, 20);
        window.drafftinkPdfPagesTruncated = pdf.numPages > count;
        if (pdf.numPages > count) console.warn(`PDF limité à ${count} pages pour préserver la mémoire du tableau.`);
        for (let number = 1; number <= count; number++) {
            const page = await pdf.getPage(number);
            const base = page.getViewport({scale: 1});
            const scale = Math.min(1.5, 2400 / Math.max(base.width, base.height));
            const viewport = page.getViewport({scale});
            canvas.width = Math.ceil(viewport.width);
            canvas.height = Math.ceil(viewport.height);
            await page.render({canvas, canvasContext: canvas.getContext('2d', {alpha: false}), viewport}).promise;
            pages.push(await new Promise((resolve, reject) => canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('PDF vers image impossible')), 'image/png')));
            page.cleanup();
        }
        canvas.width = canvas.height = 0;
        return pages;
    } finally { await pdf.destroy(); }
};
(() => {
    if (!('serviceWorker' in navigator) || !window.isSecureContext) return;
    // Existing canvas diagnostic tests deliberately bypass caching; dedicated offline tests exercise it.
    if (new URLSearchParams(location.search).has('drafftink-test') && !new URLSearchParams(location.search).has('offline-test')) return;
    let registration, noticeTimer, unavailableTimer, cacheErrorDetail, updateRequested = false, started = false;
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
    const show = text => { clearTimeout(noticeTimer); notice.textContent = text; if (!notice.isConnected) document.body.appendChild(notice); };
    const dismiss = () => {
        const button = document.createElement('button');
        button.textContent = 'Fermer';
        button.style.cssText = 'margin-left:8px;border:0;background:none;cursor:pointer';
        button.onclick = () => notice.remove();
        notice.appendChild(button);
    };
    const unavailable = detail => {
        clearTimeout(unavailableTimer);
        cacheErrorDetail = detail || cacheErrorDetail;
        show('Cache hors connexion indisponible. Envoyez tout le dossier web, puis sw.js en dernier.' + (detail ? ' ' + detail : ''));
        const retry = document.createElement('button');
        retry.textContent = 'Réessayer';
        retry.style.cssText = 'margin:6px;padding:5px 8px;cursor:pointer';
        retry.onclick = () => { started = false; start(); };
        notice.appendChild(retry);
        dismiss();
    };
    navigator.serviceWorker.addEventListener('message', event => {
        if (event.data?.type === 'Q_CACHE_ERROR') unavailable(event.data.detail);
    });
    const ready = () => {
        if (registration?.waiting) return offerUpdate();
        show(navigator.onLine ? 'Disponible hors connexion' : 'Mode hors connexion');
        dismiss();
        // No permanent overlay over canvas/panels once readiness was announced.
        noticeTimer = setTimeout(() => { if (!registration?.waiting) notice.remove(); }, 6000);
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
            const watch = worker => {
                if (!worker) return;
                let installed = worker.state === 'installed' || worker.state === 'activated';
                worker.addEventListener('statechange', () => {
                if (worker.state === 'installed') {
                    installed = true;
                    if (registration.waiting && navigator.serviceWorker.controller) offerUpdate();
                }
                if (worker.state === 'redundant' && !installed) {
                    unavailableTimer = setTimeout(() => unavailable(cacheErrorDetail), 400);
                }
                });
            };
            watch(registration.installing);
            registration.addEventListener('updatefound', () => watch(registration.installing));
            if (registration.waiting) offerUpdate();
            else navigator.serviceWorker.ready.then(ready);
            registration.update().catch(() => {});
        } catch (error) {
            console.warn('Offline cache unavailable:', error);
            unavailable(error.message);
        }
    }
    // Avoid a second WASM download while the application's first load is running.
    window.addEventListener('qraphtinc-ready', start, {once: true});
    if (document.querySelector('canvas') && !document.getElementById('loading')) start();
    window.addEventListener('online', () => { registration?.update().catch(() => {}); });
    window.addEventListener('offline', () => { if (navigator.serviceWorker.controller) ready(); });
})();
