const fs=require('fs'); const vm=require('vm'); const assert=require('node:assert/strict');
const path=require('path'); const root=path.join(__dirname,'..');
const html=fs.readFileSync(path.join(root,'web/index.html'),'utf8');
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
    if(url==='cursormouse.svg'||url==='cursortext.svg')return{ok:true,text:async()=>fs.readFileSync(path.join(root,'web',url),'utf8')};
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
  canvas.style.cursor='ew-resize';sandbox.window.drafftinkSetCursor(false,'#000000');assert.equal(canvas.style.cursor,'ew-resize');
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

