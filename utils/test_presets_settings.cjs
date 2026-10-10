// Exercise real browser storage and file selection independently of the GPU renderer.
const { chromium } = require('../work/e2e/node_modules/playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const png = path.resolve('work/e2e/fixtures/image-1200x2000.png');
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://localhost');
  if (url.pathname.endsWith('browser-tools.js')) { res.setHeader('Content-Type','text/javascript'); res.end(fs.readFileSync('web/browser-tools.js')); }
  else if (url.pathname.endsWith('presets/manifest.json')) { res.setHeader('Content-Type','application/json'); res.end(JSON.stringify({files:['trigo.png','plan_complexe.png']})); }
  else if (url.pathname.endsWith('.png')) { res.setHeader('Content-Type','image/png'); res.end(fs.readFileSync(png)); }
  else { res.setHeader('Content-Type','text/html'); res.end('<script src="browser-tools.js" defer></script><button onclick="qursoOpenNumworks()">Numworks</button>'); }
});
(async () => {
  await new Promise(resolve=>server.listen(8893,'127.0.0.1',resolve));
  const browser=await chromium.launch({headless:true});
  try {
    const context=await browser.newContext(); const page=await context.newPage();
    await page.goto('http://127.0.0.1:8893/qurso/web/');
    let presets=await page.evaluate(()=>qursoPresetList());
    assert.deepEqual(presets.map(p=>p.name).sort(),['Plan Complexe','Trigo']);
    const trigo=presets.find(p=>p.name==='Trigo');
    await page.evaluate(id=>qursoPresetRename(id,'Cercle trigo'),trigo.id);
    const plan=presets.find(p=>p.name==='Plan Complexe');
    await page.evaluate(id=>qursoPresetDelete(id),plan.id);
    const chooserEvent=page.waitForEvent('filechooser');
    const adding=page.evaluate(()=>qursoPresetAdd());
    const chooser=await chooserEvent;
    assert(chooser.isMultiple(),'Preset picker must accept multiple files');
    const buffer=fs.readFileSync(png);
    await chooser.setFiles([{name:'cube.png',mimeType:'image/png',buffer},{name:'axes.png',mimeType:'image/png',buffer}]);
    presets=await adding;
    assert.deepEqual(presets.map(p=>p.name).sort(),['Axes','Cercle trigo','Cube']);
    const blobSize=await page.evaluate(async id=>(await qursoPresetGet(id)).size,presets.find(p=>p.name==='Cube').id);
    assert.equal(blobSize,buffer.length,'PNG presets keep source bytes and resolution');
    await page.reload();
    presets=await page.evaluate(()=>qursoPresetList());
    assert.deepEqual(presets.map(p=>p.name).sort(),['Axes','Cercle trigo','Cube'],'Seed must not overwrite renames or restore deleted presets');
    const popupEvent=context.waitForEvent('page'); await page.getByRole('button',{name:'Numworks'}).click();
    const popup=await popupEvent; await popup.waitForLoadState();
    assert.equal(popup.url(),'http://127.0.0.1:8893/qurso/web/numworks/','Numworks stays relative to the installation folder');
    await browser.close();
    console.log('Preset checks passed: first-launch seed, multi-file import, original PNG bytes, persistent rename/delete, relative Numworks URL.');
  } finally { if(browser.isConnected())await browser.close(); await new Promise(resolve=>server.close(resolve)); }
})().catch(error=>{console.error(error);process.exitCode=1;server.close();});
