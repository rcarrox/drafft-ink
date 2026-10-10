const fs=require('fs'); const vm=require('vm'); const assert=require('node:assert/strict');
const path=require('path'); const root=path.join(__dirname,'..');
const html=fs.readFileSync(path.join(root,'web/index.html'),'utf8');
const splashStart=html.indexOf('        window.drafftinkLoadingDone = function()');
const splashEnd=html.indexOf('\n        // Prevent the browser',splashStart);
assert(splashStart>=0&&splashEnd>splashStart);new vm.Script(html.slice(splashStart,splashEnd));
assert(html.includes('window.drafftinkLoadingDone = function()'));
assert.equal((html.match(/loading\.remove\(\)/g)||[]).length,1,'the splash is removed only by the first-frame fade callback');
const start=html.indexOf('        // Sparse repaint scheduling:');
const end=html.indexOf('\n        // ',html.indexOf('        cursorTemplates.then',start)+20);
const helper=html.slice(start,end); new vm.Script(helper);
const database=new Map(), requests=[]; const timers=new Map(); let nextTimer=0, wakeups=0;
const canvas={style:{cursor:'auto'}};
const catalog=[{family:'Google Sans Medium',postscriptName:'GoogleSans-Medium',source:'local-server'}];
const sandbox={console,Uint8Array,ArrayBuffer,Promise,Map,Math,Number,Infinity,encodeURIComponent,
  performance:{now:()=>100}, Event:function(name){this.type=name},
  document:{querySelector:()=>canvas}, MutationObserver:class {constructor(cb){this.cb=cb}observe(){}disconnect(){}},
  setTimeout:(fn,delay)=>{const id=++nextTimer;timers.set(id,{fn,delay});return id},
  clearTimeout:id=>timers.delete(id), navigator:{permissions:{query:async()=>({state:'prompt'})}},
  window:{dispatchEvent:()=>wakeups++,queryLocalFonts:async()=>{throw Error('Permission API should not be needed')}},
  fetch:async url=>{requests.push(url);
    if(['cursormouse.svg','cursortext.svg','cursormath.svg','cursoreraser.svg','cursoreraserman.svg','cursorcrosshair.svg','cursordraw.svg'].includes(url))return{ok:true,text:async()=>fs.readFileSync(path.join(root,'web',url),'utf8')};
    if(url==='/local-fonts.json')return{ok:true,json:async()=>catalog};
    if(url==='/local-font/GoogleSans-Medium')return{ok:true,arrayBuffer:async()=>new Uint8Array([0,1,0,0]).buffer};
    return{ok:false};},
  indexedDB:{open:()=>{
    const request={};queueMicrotask(()=>{
      const db={close(){},createObjectStore(){},transaction(){const tx={objectStore(){return{
        get(key){const r={};queueMicrotask(()=>{r.result=database.get(key);r.onsuccess?.();tx.oncomplete?.()});return r},
        put(value){const r={};queueMicrotask(()=>{database.set(value.postscriptName,value);r.result=value.postscriptName;r.onsuccess?.();tx.oncomplete?.()});return r}
      }}};return tx}};
      request.result=db;request.onupgradeneeded?.();request.onsuccess?.();
    });return request;
  }}
};
vm.createContext(sandbox);vm.runInContext(helper,sandbox);
(async()=>{
  await new Promise(resolve=>setImmediate(resolve));
  sandbox.window.drafftinkSetCursor(false,'#0080ff');
  let match=canvas.style.cursor.match(/^url\("data:image\/svg\+xml,([^"\n]+)"\)/);assert(match);
  const mouse=decodeURIComponent(match[1]);assert(mouse.includes('fill="#ffffff"'));assert(mouse.includes('stroke="#0080ff"'));assert(mouse.includes('feDropShadow'));
  sandbox.window.drafftinkSetCursor(true,'#0080ff');assert(canvas.style.cursor.endsWith('14 14, text'));
  sandbox.window.drafftinkSetCursor(2,'#0080ff');
  assert(decodeURIComponent(canvas.style.cursor).includes('M60 26l12 12'));
  sandbox.window.drafftinkSetCursor(3,'#0080ff');
  assert(canvas.style.cursor.endsWith('4 14, default'));
  assert(decodeURIComponent(canvas.style.cursor).includes('M8.086 2.207'));
  sandbox.window.drafftinkSetCursor(4,'#0080ff');
  assert(canvas.style.cursor.endsWith('3 21, default'));
  assert(decodeURIComponent(canvas.style.cursor).includes('M18.62 1.5'));
  sandbox.window.drafftinkSetCursor(5,'#0080ff');
  assert(canvas.style.cursor.endsWith('12 12, crosshair'));
  assert(decodeURIComponent(canvas.style.cursor).includes('M11 3h2v6'));
  sandbox.window.drafftinkSetCursor(6,'#0080ff');
  assert(canvas.style.cursor.endsWith('4 14, default'));
  assert(decodeURIComponent(canvas.style.cursor).includes('M8.086 2.207'));
  canvas.style.cursor='pointer';sandbox.window.drafftinkSetCursor(0,'#000000',true);assert(canvas.style.cursor.endsWith('6 5, default'));
  const date=new Date(2026,9,8,11,26,45);
  assert.equal(sandbox.window.drafftinkSnapshotFilename('Canvas-x01','png',date),'2026-10-08_112645 Canvas-x01.png');
  assert.equal(sandbox.window.drafftinkSnapshotFilename('Canvas-x01','png',date),'2026-10-08_112646 Canvas-x01.png');
  canvas.style.cursor='ew-resize';sandbox.window.drafftinkSetCursor(false,'#000000',false);assert.equal(canvas.style.cursor,'ew-resize');
  sandbox.window.drafftinkSetCursor(4,'#000000',true,'nwse-resize');
  assert.equal(canvas.style.cursor,'nwse-resize','Time resize wins over tool and UI cursors');
  canvas.style.cursor='default';
  sandbox.window.drafftinkSetCursor(4,'#000000',true,'nwse-resize');
  assert.equal(canvas.style.cursor,'nwse-resize','A native cursor reset must not replace an active handle cursor');
  sandbox.window.drafftinkSetCursor(4,'#000000',false,'');
  assert(canvas.style.cursor.endsWith('3 21, default'),'Leaving the handle restores the Draw cursor');
  sandbox.window.drafftinkRequestRepaint(500);sandbox.window.drafftinkRequestRepaint(1000);assert.equal(timers.size,1);
  assert.equal([...timers.values()][0].delay,500);
  sandbox.window.drafftinkRequestRepaint(0);assert.equal(timers.size,1);assert.equal([...timers.values()][0].delay,0);
  [...timers.values()][0].fn();assert.equal(wakeups,1);
  const fonts=await sandbox.window.drafftinkQueryLocalFonts();assert.equal(fonts[0].postscriptName,'GoogleSans-Medium');
  const font=await sandbox.window.drafftinkReadLocalFont('GoogleSans-Medium',false);assert.equal(font.bytes.length,4);
  const before=requests.filter(r=>r==='/local-font/GoogleSans-Medium').length;
  await sandbox.window.drafftinkReadLocalFont('GoogleSans-Medium',false);
  assert.equal(requests.filter(r=>r==='/local-font/GoogleSans-Medium').length,before);
  await assert.rejects(sandbox.window.drafftinkReadLocalFont('missing-font',false));
  console.log('6 browser-helper checks passed: SVG/outline, mode cursor, resize cursor, sparse repaint, Windows font catalog, cached font restore.');
})().catch(error=>{console.error(error);process.exitCode=1});

