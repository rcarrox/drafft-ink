const {chromium}=require('../work/e2e/node_modules/playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const evidence=path.resolve('work/e2e/evidence');fs.mkdirSync(evidence,{recursive:true});
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--enable-unsafe-webgpu','--use-angle=swiftshader']});
 const report={scenario:'two canvases, one 1200x2000 PNG per canvas, pinned image, zoom-out, animated stopwatch',environment:'GitHub Chromium software WebGPU; excludes total Edge/Windows process RAM'};
 async function measure(label,url){
  const context=await browser.newContext({viewport:{width:1280,height:720}});
  await context.addInitScript(()=>{
   localStorage.setItem('drafftink.user_settings.v1',JSON.stringify({restore_last_document:false,autosave_enabled:false,intro_json:'',default_font:'Noto Sans',default_font_postscript:'',hide_properties:true}));
   window.__qursoCompute=0;window.__qursoComputeSupported=false;
   try{const proto=window.GPUComputePassEncoder?.prototype;if(proto){for(const name of ['dispatchWorkgroups','dispatchWorkgroupsIndirect']){const original=proto[name];if(original)proto[name]=function(...args){window.__qursoCompute++;return Reflect.apply(original,this,args);};}window.__qursoComputeSupported=true;}}catch{}
  });
  const page=await context.newPage();
  const state=()=>page.evaluate(()=>JSON.parse(window.__drafftinkTestState));
  const wait=async predicate=>{for(let i=0;i<180;i++){const s=await state().catch(()=>null);if(s&&predicate(s))return s;await page.waitForTimeout(100);}throw Error(label+' benchmark timeout: '+JSON.stringify(await state(),(key,value)=>key==='data'?'[image bytes omitted]':value));};
  const click=async name=>{const s=await wait(s=>s.controls[name]);const r=s.controls[name];await page.mouse.move((r[0]+r[2])/2,(r[1]+r[3])/2);await page.waitForTimeout(150);await page.mouse.click((r[0]+r[2])/2,(r[1]+r[3])/2,{delay:80});await page.waitForTimeout(150);};
  const importImage=async()=>{await click('tool_Insert');const event=page.waitForEvent('filechooser');await click('Insert Image');await(await event).setFiles('work/e2e/fixtures/image-1200x2000.png');await wait(s=>s.shapes.some(s=>s.shape.Image));};
  try{
   await page.goto(url);await wait(s=>s.shapes.length===0);
   await importImage();await click('New canvas');await wait(s=>s.tabs===2&&s.shapes.length===0);await importImage();
   await page.mouse.move(420,375);await page.waitForTimeout(150);await page.keyboard.press('s');await page.mouse.click(420,375,{delay:80});await wait(s=>s.selected_count===1);
   await page.keyboard.press('Control+l');await wait(s=>s.shapes[0].pinned);
   const beforeZoom=await state();
   for(let i=0;i<20;i++){await page.keyboard.press('-');await page.waitForTimeout(60);}
   const afterZoom=await wait(s=>s.zoom<=0.1001);
   if(label==='current')assert.equal(afterZoom.memory.image_cache_bytes,beforeZoom.memory.image_cache_bytes,'Pinned pixels must not vary with camera zoom');
   await click('tool_Insert');await click('Insert Time');
   let s=await wait(s=>Object.keys(s.controls).some(k=>k.startsWith('Time widget ')&&!k.startsWith('Time widget size')));
   const key=Object.keys(s.controls).find(k=>k.startsWith('Time widget ')&&!k.startsWith('Time widget size'));
   let r=s.controls[key];await page.mouse.move((r[0]+r[2])/2,(r[1]+r[3])/2);await page.waitForTimeout(180);await page.mouse.click((r[0]+r[2])/2,(r[1]+r[3])/2,{delay:80});await page.waitForTimeout(180);
   r=(await state()).controls[key];await page.mouse.click(r[0]+25,r[3]-25,{delay:80});
   await page.mouse.move(1000,100);await page.waitForTimeout(350);
   const cdp=await context.newCDPSession(page);await cdp.send('Performance.enable');
   const metrics=async()=>Object.fromEntries((await cdp.send('Performance.getMetrics')).metrics.map(m=>[m.name,m.value]));
   const a=await metrics(),counter=await page.evaluate(()=>window.__qursoCompute),renderA=(await state()).canvas_render_count;
   await page.waitForTimeout(1200);
   const b=await metrics(),compute=await page.evaluate(counter=>({count:window.__qursoCompute-counter,supported:window.__qursoComputeSupported}),counter);
   const final=await state();
   if(label==='current'){assert(final.time_widgets[0].running,'Stopwatch must be actively animating');assert.equal(final.canvas_render_count,renderA,'Static canvas should not render for clock animation');if(compute.supported)assert.equal(compute.count,0,'Clock-only frames must not dispatch Vello compute');}
   return {tabs:final.tabs,active_images:final.shapes.filter(s=>s.shape.Image).length,preview_cache_bytes_before_zoom:beforeZoom.memory.image_cache_bytes,preview_cache_bytes_after_zoom:afterZoom.memory.image_cache_bytes,wasm_bytes:(await page.evaluate(()=>window.drafftinkMemoryUsage())).wasm_bytes,js_heap_used_bytes:b.JSHeapUsedSize,task_seconds:b.TaskDuration-a.TaskDuration,gpu_compute_dispatches:compute.supported?compute.count:null,canvas_frames:renderA===undefined?null:final.canvas_render_count-renderA,image_decodes:final.image_decode_count??null};
  }finally{await context.close();}
 }
 try{
  if(fs.existsSync('work/e2e/baseline-web/index.html'))report.baseline_0_31=await measure('baseline','http://127.0.0.1:8894/?drafftink-test=1');
  else report.baseline_status='Previous artifact unavailable; current assertions still run.';
  report.current_0_32=await measure('current','http://127.0.0.1:8888/?drafftink-test=1');
  if(report.baseline_0_31?.gpu_compute_dispatches!==null&&report.baseline_0_31?.gpu_compute_dispatches>0){report.compute_dispatches_saved=report.baseline_0_31.gpu_compute_dispatches-report.current_0_32.gpu_compute_dispatches;}
  fs.writeFileSync(path.join(evidence,'render-performance-0.32.0.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report));
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
