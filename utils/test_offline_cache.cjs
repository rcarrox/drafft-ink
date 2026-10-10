// Actual browser service worker, nested FTP path, no network, and safe updates.
const {chromium}=require('../work/e2e/node_modules/playwright');
const fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const assert=require('node:assert/strict'),{execFileSync}=require('node:child_process');
const root=path.resolve('work/e2e/offline-site');
fs.mkdirSync(root,{recursive:true});fs.cpSync('web',root,{recursive:true});
const indexPath=path.join(root,'index.html');
const sourceIndex=fs.readFileSync(indexPath,'utf8');
const build=()=>execFileSync('python3',['utils/build_offline_cache.py','--web-dir',root]);build();
// Simulate FTP ASCII mode after a complete upload: the manifest is generated
// from LF files, while index.html reaches the browser with CRLF line endings.
fs.writeFileSync(indexPath,sourceIndex.replace(/\r?\n/g,'\r\n'));
const prefix='/qrapht/web/';
const server=http.createServer((request,response)=>{
  const name=new URL(request.url,'http://localhost').pathname;
  if(!name.startsWith(prefix)){response.writeHead(404);response.end('Unrelated site');return;}
  const file=path.resolve(root,name.slice(prefix.length)||'index.html');
  if(!file.startsWith(root+path.sep)||!fs.existsSync(file)||fs.statSync(file).isDirectory()){response.writeHead(404);response.end();return;}
  const mime={'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.svg':'image/svg+xml'};
  response.writeHead(200,{'Content-Type':mime[path.extname(file)]||'application/octet-stream','Cache-Control':'no-cache'});
  fs.createReadStream(file).pipe(response);
});
(async()=>{
  await new Promise(resolve=>server.listen(8891,'127.0.0.1',resolve));
  const browser=await chromium.launch({headless:true,args:['--enable-unsafe-webgpu','--use-angle=swiftshader']});
  const context=await browser.newContext({viewport:{width:1280,height:720}});
  await context.addInitScript(()=>{
    // GPU initialization may outlast the short readiness notice. Observe the
    // actual visible DOM announcement from page startup, not after canvas load.
    window.__offlineNoticeSeen=false;
    new MutationObserver(()=>{
      const notice=document.getElementById('qraphtinc-offline');
      if(notice?.isConnected&&!notice.hidden&&notice.textContent.includes('Disponible hors connexion'))window.__offlineNoticeSeen=true;
    }).observe(document,{subtree:true,childList:true,characterData:true});
    if(!localStorage.getItem('drafftink.user_settings.v1'))localStorage.setItem('drafftink.user_settings.v1',JSON.stringify({restore_last_document:false,autosave_enabled:false,default_font:'Noto Sans',default_font_postscript:'',accent_color:[180,35,100]}));
  });
  const page=await context.newPage(),logs=[];
  page.on('console',message=>logs.push(message.type()+': '+message.text()));
  const url='http://127.0.0.1:8891'+prefix;
  const loaded=async()=>{
    await page.waitForFunction(()=>!!document.querySelector('canvas')&&!document.getElementById('loading'),null,{timeout:60000});
    await page.waitForFunction(()=>!!navigator.serviceWorker.controller,null,{timeout:60000});
  };
  try{
    await page.goto(url);await loaded();assert.equal(await page.title(),'Qurso🌿');
    await page.waitForFunction(()=>window.__offlineNoticeSeen===true,null,{timeout:60000});
    assert.equal(await page.evaluate(()=>navigator.serviceWorker.controller.scriptURL),url+'sw.js');
    assert.deepEqual(await page.evaluate(async()=>{
      const names=await caches.keys(),key=names.find(n=>n.startsWith('qraphtinc-offline:'));
      const cache=await caches.open(key);
      return [await cache.match(new URL('pdfjs/pdf.min.js',location.href))!==undefined,
        await cache.match(new URL('pdfjs/pdf.worker.min.js',location.href))!==undefined];
    }),[true,true]);
    await page.evaluate(async()=>{
      localStorage.setItem('offline-user-proof','preserved');
      const other=await caches.open('unrelated-site-cache');await other.put('/unrelated-proof',new Response('keep'));
      await new Promise((resolve,reject)=>{
        const request=indexedDB.open('offline-user-document-proof',1);
        request.onupgradeneeded=()=>request.result.createObjectStore('documents');
        request.onerror=()=>reject(request.error);
        request.onsuccess=()=>{const db=request.result,tx=db.transaction('documents','readwrite');tx.objectStore('documents').put('local document','canvas');tx.oncomplete=()=>{db.close();resolve();};};
      });
    });
    await context.setOffline(true);await page.reload();await loaded();
    assert.equal(await page.title(),'Qurso🌿');
    assert.equal(await page.evaluate(async()=>new Uint8Array(await (await fetch('./pkg/drafftink_app_bg.wasm')).arrayBuffer())[1]),97);
    // A worker from a partial FTP upload must fail without replacing the good cache.
    const indexFile=indexPath,original=fs.readFileSync(indexFile,'utf8');
    const newer=original.replace('<title>Qurso🌿</title>','<title>Qurso🌿 update test</title>');
    fs.writeFileSync(indexFile,newer);build();fs.writeFileSync(indexFile,original);
    // Prepare the partial upload before reconnecting; the online event also
    // checks for updates, and must not race a still-old server response.
    await page.evaluate(async()=>{
      window.__offlineRejectedBuild=false;
      const r=await navigator.serviceWorker.getRegistration();
      r.addEventListener('updatefound',()=>{
        const candidate=r.installing;
        candidate?.addEventListener('statechange',()=>{if(candidate.state==='redundant')window.__offlineRejectedBuild=true;});
      });
    });
    await context.setOffline(false);
    await page.evaluate(async()=>{const r=await navigator.serviceWorker.getRegistration();await r.update();});
    await page.waitForFunction(()=>window.__offlineRejectedBuild===true,null,{timeout:60000});
    await context.setOffline(true);await page.reload();await loaded();assert.equal(await page.title(),'Qurso🌿');
    // Finish upload; the new release waits and leaves this sheet running.
    fs.writeFileSync(indexFile,newer);await context.setOffline(false);
    await page.evaluate(async()=>{const r=await navigator.serviceWorker.getRegistration();await r.update();});
    await page.waitForFunction(async()=>!!(await navigator.serviceWorker.getRegistration())?.waiting,null,{timeout:60000});
    assert.equal(await page.title(),'Qurso🌿');
    page.once('dialog',dialog=>dialog.accept());
    await page.getByRole('button',{name:'Installer et recharger'}).click();
    await page.waitForFunction(()=>document.title==='Qurso🌿 update test',null,{timeout:60000});await loaded();
    await context.setOffline(true);await page.reload();await loaded();assert.equal(await page.title(),'Qurso🌿 update test');
    assert.equal(await page.evaluate(()=>localStorage.getItem('offline-user-proof')),'preserved');
    assert.deepEqual(await page.evaluate(()=>JSON.parse(localStorage.getItem('drafftink.user_settings.v1')).accent_color),[180,35,100]);
    assert.equal(await page.evaluate(()=>new Promise((resolve,reject)=>{
      const request=indexedDB.open('offline-user-document-proof',1);request.onerror=()=>reject(request.error);
      request.onsuccess=()=>{const db=request.result,read=db.transaction('documents').objectStore('documents').get('canvas');read.onsuccess=()=>{db.close();resolve(read.result);};};
    })),'local document');
    const cachesLeft=await page.evaluate(()=>caches.keys());
    assert(cachesLeft.includes('unrelated-site-cache'));
    assert.equal(cachesLeft.filter(key=>key.startsWith('qraphtinc-offline:'+prefix+':')).length,1);
    const evidence=path.resolve('work/e2e/evidence');fs.mkdirSync(evidence,{recursive:true});
    fs.writeFileSync(path.join(evidence,'offline-result.json'),JSON.stringify({passed:true,scope:prefix,offline_reload:true,wasm_cached:true,incomplete_upload_rejected:true,update_requires_click:true,offline_updated_release:true,preferences_and_documents_preserved:true,unrelated_site_cache_preserved:true},null,2));
    console.log('Offline cache browser checks passed: nested scope, offline reload/WASM, partial upload integrity, explicit update, preserved user storage and unrelated caches.');
  }catch(error){
    fs.mkdirSync('work/e2e/evidence',{recursive:true});
    const debug=await page.evaluate(async()=>({title:document.title,notice:document.getElementById('qraphtinc-offline')?.textContent,readySeen:window.__offlineNoticeSeen,rejectedBuild:window.__offlineRejectedBuild,caches:await caches.keys()})).catch(()=>null);
    fs.writeFileSync('work/e2e/evidence/offline-failure.json',JSON.stringify(debug,null,2));
    throw error;
  }finally{
    fs.mkdirSync('work/e2e/evidence',{recursive:true});fs.writeFileSync('work/e2e/evidence/offline-browser.log',logs.join('\n'));
    await browser.close();await new Promise(resolve=>server.close(resolve));
  }
})().catch(error=>{console.error(error);process.exitCode=1;server.close();});
