let chromium;
try { ({ chromium } = require('../work/e2e/node_modules/playwright')); }
catch { ({ chromium } = require('playwright')); }
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const assert = require('node:assert/strict');

const root = path.resolve('web');
const html = fs.readFileSync(path.join(root, 'index.html'), 'utf8');
assert(!html.includes('offline.js'), 'the application must not load the offline-cache installer');
assert(!html.includes("serviceWorker.register"), 'the application must not install a service worker');
assert(html.includes("addEventListener('beforeunload'"), 'refresh/navigation should request confirmation');
assert(html.includes('Loading Qurso'), 'the loading label should identify Qurso');

const server = http.createServer((request, response) => {
  const requested = new URL(request.url, 'http://localhost').pathname;
  const file = path.resolve(root, requested === '/' ? 'index.html' : `.${requested}`);
  if (!file.startsWith(root + path.sep) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
    response.writeHead(404); response.end(); return;
  }
  const mime = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
  response.writeHead(200, { 'Content-Type': mime[path.extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-cache' });
  fs.createReadStream(file).pipe(response);
});

(async () => {
  await new Promise(resolve => server.listen(8892, '127.0.0.1', resolve));
  const browser = await chromium.launch({ headless: true, args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'] });
  try {
    const page = await browser.newPage();
    page.on('dialog', dialog => {
      dialog.accept();
    });
    await page.goto('http://127.0.0.1:8892/', { waitUntil: 'domcontentloaded', timeout: 60000 });
    assert.equal(await page.title(), 'Qurso🌿');
    assert.equal(await page.evaluate(() => navigator.serviceWorker?.controller ?? null), null);
    assert.equal(await page.locator('#qraphtinc-offline').count(), 0);
    await page.evaluate(async () => {
      await new Promise((resolve, reject) => {
        const request = indexedDB.open('qurso-online-storage-check', 1);
        request.onupgradeneeded = () => request.result.createObjectStore('boards');
        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
          const db = request.result, transaction = db.transaction('boards', 'readwrite');
          transaction.objectStore('boards').put('saved board', 'last');
          transaction.oncomplete = () => { db.close(); resolve(); };
        };
      });
    });
    const unload = await page.evaluate(() => {
      const event = new Event('beforeunload', { cancelable: true });
      const allowed = window.dispatchEvent(event);
      return { allowed, returnValue: event.returnValue };
    });
    assert.equal(unload.allowed, false, 'beforeunload must request confirmation');
    assert.equal(unload.returnValue, false, 'beforeunload should set its cancellation value');
    await page.reload({ waitUntil: 'domcontentloaded' });
    const restored = await page.evaluate(() => new Promise((resolve, reject) => {
      const request = indexedDB.open('qurso-online-storage-check', 1);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const db = request.result, read = db.transaction('boards').objectStore('boards').get('last');
        read.onsuccess = () => { db.close(); resolve(read.result); };
      };
    }));
    assert.equal(restored, 'saved board', 'IndexedDB remains independent from service workers');
    console.log('Online-only browser checks passed: no offline installer, unload confirmation, IndexedDB survives reload.');
  } finally {
    await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode = 1; server.close(); });
