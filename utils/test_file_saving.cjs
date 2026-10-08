const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const html=fs.readFileSync('web/index.html','utf8');
const start=html.indexOf('        const snapshotTimes');
const end=html.indexOf('        // The app uses WebGPU.',start);
const code=html.slice(start,end);new vm.Script(code);
let permission='granted',prompts=0,filename='',saved=null;const store=new Map(),files=new Map();
const handle={name:'Fixture',queryPermission:async()=>permission,requestPermission:async()=>{prompts++;return permission;},getFileHandle:async(name,options={})=>{
 if(!options.create&&!files.has(name)){const error=new Error('missing');error.name='NotFoundError';throw error;}
 filename=name;return{createWritable:async()=>({write:async blob=>{saved=blob;files.set(name,blob);},close:async()=>{},abort:async()=>{}})};
}};store.set('exportDirectory',handle);
const context={console,Promise,Blob,document:{addEventListener(){}},window:{drafftinkRequestRepaint(){},showDirectoryPicker:async()=>handle},indexedDB:{open(){const request={};queueMicrotask(()=>{
 request.result={objectStoreNames:{contains:()=>true},close(){},transaction(){const tx={objectStore(){return{get(key){const r={};queueMicrotask(()=>{r.result=store.get(key);r.onsuccess();});return r;},put(value,key){store.set(key,value);queueMicrotask(()=>tx.oncomplete());}}}};return tx;}};
 request.onupgradeneeded?.();request.onsuccess();});return request;}}};vm.createContext(context);vm.runInContext(code,context);
(async()=>{await new Promise(resolve=>setImmediate(resolve));assert.equal(context.window.drafftinkExportFolderReady,true);
 assert.equal(await context.window.drafftinkSaveBlobToExportDirectory('bad:name.png','PNG',true),true);assert.equal(filename,'bad_name.png');assert.equal(saved,'PNG');
 permission='prompt';prompts=0;assert.equal(await context.window.drafftinkSaveBlobToExportDirectory('auto.png','PNG',true),false);assert.equal(prompts,0,'Background save must not prompt or download');assert.match(context.window.drafftinkSaveStatus,/Réautoriser/);
 permission='granted';assert.equal(await context.window.drafftinkPickExportDirectory(),'Fixture');assert.equal(context.window.drafftinkExportFolderReady,true);
 assert.equal(await context.window.drafftinkSaveBlobToExportDirectory('resume.png','PNG',true),true);assert.equal(context.window.drafftinkDiskSaveBusy,false);
 const name=context.window.drafftinkSnapshotFilename('Canvas','png',new Date(2026,9,8,11,26,45));
 files.set(name,'old PNG');files.set(name.replace('.png','.json'),'old JSON');
 assert.equal(await context.window.drafftinkSaveSnapshotPair(name,'NEW PNG','{"canvas":1}'),true);
 assert.equal(files.get(name),'old PNG');assert.equal(files.get(name.replace('.png','.json')),'old JSON');
 const next='2026-10-08_112646 Canvas.png';assert.equal(files.get(next),'NEW PNG');
 assert.equal(await files.get(next.replace('.png','.json')).text(),'{"canvas":1}');
 console.log('PNG directory save checks passed: actual write, filename, background permission, reauthorization.');})().catch(error=>{console.error(error);process.exitCode=1});
