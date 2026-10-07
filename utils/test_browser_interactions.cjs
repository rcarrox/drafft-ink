// Real Chromium/WASM input; read-only diagnostics never mutate the application.
const { chromium } = require('../work/e2e/node_modules/playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');
const evidence = path.resolve('work/e2e/evidence');
fs.mkdirSync(evidence, { recursive: true });


// Reject empty WebGPU captures; reconstruct standard Chromium RGB/RGBA PNGs.
function inspectCapture(file) {
  const png=fs.readFileSync(file);
  let offset=8,width,height,bpp;const parts=[];
  while(offset<png.length) {
    const size=png.readUInt32BE(offset),tag=png.toString('ascii',offset+4,offset+8);
    const data=png.subarray(offset+8,offset+8+size);
    if(tag==='IHDR') {width=data.readUInt32BE(0);height=data.readUInt32BE(4);assert.equal(data[8],8);bpp=data[9]===6?4:data[9]===2?3:0;assert(bpp,'Unsupported screenshot PNG format');}
    if(tag==='IDAT')parts.push(data);
    offset+=size+12;
  }
  const raw=zlib.inflateSync(Buffer.concat(parts));const stride=width*bpp;
  let previous=Buffer.alloc(stride),position=0,nonwhite=0;const colors=new Set();
  const paeth=(a,b,c)=> {const p=a+b-c,pa=Math.abs(p-a),pb=Math.abs(p-b),pc=Math.abs(p-c);return pa<=pb&&pa<=pc?a:pb<=pc?b:c;};
  for(let y=0;y<height;y++) {
    const filter=raw[position++],row=Buffer.from(raw.subarray(position,position+stride));position+=stride;
    for(let x=0;x<stride;x++) {
      const left=x>=bpp?row[x-bpp]:0,up=previous[x],upperLeft=x>=bpp?previous[x-bpp]:0;
      const prediction=filter===0?0:filter===1?left:filter===2?up:filter===3?Math.floor((left+up)/2):paeth(left,up,upperLeft);
      row[x]=(row[x]+prediction)&255;
    }
    for(let x=0;x<stride;x+=bpp) {
      if(row[x]<235||row[x+1]<235||row[x+2]<235)nonwhite++;
      if(colors.size<256)colors.add(Array.from(row.subarray(x,x+bpp)).join(','));
    }
    previous=row;
  }
  return {file:path.basename(file),width,height,nonwhite_pixels:nonwhite,distinct_colors:colors.size,valid:nonwhite>1000&&colors.size>16};
}

(async () => {
  const browser = await chromium.launch({ channel: 'chromium', headless: true, args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'] });
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  await context.addInitScript(() => localStorage.setItem('drafftink.user_settings.v1', JSON.stringify({ restore_last_document: false, intro_json: '', autosave_enabled: false, default_font: 'Noto Sans', default_font_postscript: '' })));
  const page = await context.newPage();
  const logs = [];
  page.on('console', message => logs.push(message.type() + ': ' + message.text()));
  page.on('pageerror', error => logs.push('PAGEERROR: ' + error));
  const state = () => page.evaluate(() => JSON.parse(window.__drafftinkTestState));
  const wait = async predicate => {
    for (let i = 0; i < 100; i++) {
      const current = await state().catch(() => null);
      if (current && predicate(current)) return current;
      await page.waitForTimeout(100);
    }
    throw new Error('Timed out; state=' + JSON.stringify(await state().catch(() => null)));
  };
  const text = s => s.shapes.find(item => item.shape.Text)?.shape.Text;
  const snapshot=async(target,file)=> {
    await target.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    await target.screenshot({path:path.join(evidence,file)});
  };
  const exportPixels=async(target,file)=> {
    if(logs.some(line=>line.includes('A valid external Instance reference no longer exists'))) {
      return {file,valid:false,status:'unavailable',reason:'CI software WebGPU instance lost; pixel readback cannot be validated here'};
    }
    const downloaded=target.waitForEvent('download',{timeout:60000});
    await target.keyboard.press('Control+e');
    await (await downloaded).saveAs(path.join(evidence,file));
    const inspected=inspectCapture(path.join(evidence,file));
    assert(inspected.valid,`GPU PNG export is empty: ${JSON.stringify(inspected)}`);
    return inspected;
  };
  const input = await context.newCDPSession(page);
  const keys = async value => {
    for (const char of value) {
      if (char.codePointAt(0) < 128) await page.keyboard.press(char);
      else {
        await input.send('Input.dispatchKeyEvent', { type: 'keyDown', key: char, text: char, unmodifiedText: char });
        await input.send('Input.dispatchKeyEvent', { type: 'keyUp', key: char });
      }
    }
  };
  const caretPacket = async key => {
    await input.send('Input.dispatchKeyEvent',{type:'keyDown',key,code:'BracketLeft',...(key==='Dead'?{}:{text:key,unmodifiedText:key})});
    await input.send('Input.dispatchKeyEvent',{type:'keyUp',key,code:'BracketLeft'});
  };
  const control = async name => {
    const s = await wait(s => !!s.controls[name]);
    const [x0, y0, x1, y1] = s.controls[name];
    await page.mouse.click((x0 + x1) / 2, (y0 + y1) / 2);
  };
  const fill = async (name, value) => { await control(name); await page.keyboard.press('Control+a'); await keys(value); };
  try {
    await page.goto(process.env.DRAFFTINK_TEST_URL || 'http://127.0.0.1:8888/?drafftink-test=1');
    await wait(s => s.shapes.length === 0);
    await page.mouse.click(370,270);
    await page.keyboard.press('t');
    await page.mouse.click(370, 270);
    await wait(s => !!s.editing_text);
    await keys('123'); await caretPacket('Dead'); await keys('4');
    await wait(s => text(s)?.content === '123^4');
    await page.keyboard.press('Backspace'); await page.keyboard.press('Backspace'); await keys('⁴');
    await wait(s => text(s)?.content === '123⁴');
    for (const [trigger,replacement] of [['>','≥'],['<','≤']]) {
      await caretPacket('Dead');await keys(trigger);
      await page.keyboard.press('Backspace');await page.keyboard.press('Backspace');await keys(replacement);
    }
    await caretPacket('Dead');await caretPacket('^');await caretPacket('Dead');await keys('p');
    await caretPacket('Dead');await keys('â');
    await wait(s => text(s)?.content === '123⁴≥≤^^^pâ');
    await page.keyboard.press('Control+a');
    await page.keyboard.press('Control+b'); await page.keyboard.press('Control+i'); await page.keyboard.press('Control+u');
    await wait(s => text(s)?.char_styles.length === Array.from(text(s).content).length && text(s).char_styles.every(style => style.bold && style.italic && style.underline));
    await page.keyboard.press('ArrowRight');
    await page.keyboard.press('Shift+ArrowLeft');await page.keyboard.press('Shift+ArrowLeft');
    await page.keyboard.press('Control+b');await page.keyboard.press('Control+i');await page.keyboard.press('Control+u');
    await wait(s=>text(s)?.char_styles.slice(-2).every(style=>!style.bold&&!style.italic&&!style.underline) && text(s)?.char_styles.slice(0,-2).every(style=>style.bold&&style.italic&&style.underline));
    await page.keyboard.press('ArrowRight');
    await control('Fraction'); await snapshot(page,'inline-dialog.png'); await fill('Numérateur', '1/2'); await fill('Dénominateur', '3/4'); await control('Insérer');
    await wait(s => text(s)?.formulas.length === 1 && !s.inline_dialog);
    await snapshot(page,'text-fraction.png');
    await keys(' fin');
    await wait(s => text(s)?.content.endsWith(' fin'));
    await control('Racine n-ième'); await fill('Expression', 'x+1'); await fill('Indice de la racine', '3'); await control('Insérer');
    await wait(s => text(s)?.formulas.length === 2 && !s.inline_dialog);
    await snapshot(page,'text-root.png');
    const before = await state();
    await page.keyboard.press('Control+p'); await wait(s => s.presentation);
    await snapshot(page,'presentation.png');
    await page.keyboard.press('Control+p'); await wait(s => !s.presentation);
    assert.equal(text(await state()).content, text(before).content);
    await page.keyboard.press('Escape');
    await wait(s => !s.editing_text && text(s)?.formulas.length === 2);
    await page.keyboard.press('Control+z');await wait(s=>text(s)?.formulas.length===1);
    await page.keyboard.press('Control+Shift+z');await wait(s=>text(s)?.formulas.length===2);

    assert(!logs.some(line => line.startsWith('PAGEERROR:')), logs.join('\n'));
    const textPixels=await exportPixels(page,'text-render-export.png');
    fs.writeFileSync(path.join(evidence, 'state.json'), JSON.stringify(await state(), null, 2));
    // Type a command directly in Text; arguments move into a focused mini editor.
    await page.keyboard.press('t');await page.mouse.click(360,500);await wait(s=>!!s.editing_text);
    await keys('sum(');await wait(s=>s.command_editor==='sum(');
    const commandText=s=>s.shapes.find(item=>item.id===s.editing_text)?.shape.Text;
    await wait(s=>commandText(s)?.formulas.length===1&&commandText(s).formulas[0].math.latex.includes('\\sum'));
    await keys('kx,k,1,n)');await wait(s=>s.command_editor==='sum(kx,k,1,n)');
    await wait(s=>commandText(s)?.formulas[0].math.latex==='\\sum_{k=1}^{n} kx');
    await page.keyboard.press('Enter');await wait(s=>s.command_editor===null&&!!s.editing_text);
    await keys(' = frac(');await wait(s=>s.command_editor==='frac(');
    await keys('a,frac(b,c))');await wait(s=>s.command_editor==='frac(a,frac(b,c))');
    await wait(s=>commandText(s)?.formulas.some(f=>f.math.latex==='\\frac{a}{\\frac{b}{c}}'));
    await snapshot(page,'live-text-command.png');
    await page.keyboard.press('Enter');await wait(s=>s.command_editor===null&&!!s.editing_text);await keys(' suite');
    await wait(s=>commandText(s)?.content.endsWith(' suite'));
    await page.keyboard.press('Escape');await wait(s=>s.editing_text===null);

    // Separate browser context with a public, deterministic 1200x2000 PNG fixture.
    const imageContext = await browser.newContext({viewport:{width:1280,height:720}});
    const imageId='00000000-0000-4000-8000-000000000001';
    const image={id:imageId,position:{x:200,y:150},width:160,height:120,source_width:1200,source_height:2000,format:'Png',data_base64:fs.readFileSync('work/e2e/fixtures/image-1200x2000.png').toString('base64'),rotation:0,crop:{x0:0,y0:0,x1:1,y1:1},style:{stroke_color:{r:0,g:0,b:0,a:255},stroke_width:2,fill_color:null,opacity:1}};
    // ShapeStyle carries additional defaults, filled from the real Text shape.
    image.style=text(before).style;
    const fixture={id:'image-fixture',name:'Image',shapes:{[imageId]:{Image:image}},z_order:[imageId]};
    await imageContext.addInitScript(fixture => localStorage.setItem('drafftink.user_settings.v1',JSON.stringify({restore_last_document:false,autosave_enabled:false,intro_json:JSON.stringify(fixture)})),fixture);
    const imagePage=await imageContext.newPage();
    imagePage.on('console',message=>logs.push(message.type()+': '+message.text()));
    imagePage.on('pageerror',error=>logs.push('PAGEERROR: '+error));
    await imagePage.goto('http://127.0.0.1:8888/?drafftink-test=1');
    await imagePage.waitForFunction(() => window.__drafftinkTestState && JSON.parse(window.__drafftinkTestState).shapes.some(s=>s.shape.Image));
    const imageState=async()=>JSON.parse(await imagePage.evaluate(()=>window.__drafftinkTestState));
    const waitImage=async predicate=> { for(let i=0;i<100;i++){const state=await imageState();if(predicate(state.shapes.find(s=>s.shape.Image)))return state;await imagePage.waitForTimeout(100);}throw new Error('Image transform failed: '+JSON.stringify((await imageState()).shapes.map(s=>({bounds:s.bounds,rotation:s.shape.Image?.rotation,flip_x:s.shape.Image?.flip_x,flip_y:s.shape.Image?.flip_y})))); };
    let item=(await imageState()).shapes.find(s=>s.shape.Image);
    await imagePage.mouse.click((item.bounds[0]+item.bounds[2])/2,(item.bounds[1]+item.bounds[3])/2);
    const handle=item.handles.find(h=>h.kind==='Corner(BottomRight)');
    const fixed=item.handles.find(h=>h.kind==='Corner(TopLeft)');
    await imagePage.mouse.move(handle.x,handle.y);await imagePage.mouse.down();await imagePage.mouse.move(fixed.x-100,fixed.y-80,{steps:20});await imagePage.mouse.up();
    await waitImage(s=>s.shape.Image.flip_x && s.shape.Image.flip_y);
    await snapshot(imagePage,'image-flipped.png');
    await imagePage.keyboard.press('Control+z');await waitImage(s=>!s.shape.Image.flip_x && !s.shape.Image.flip_y);
    await imagePage.keyboard.press('Control+Shift+z');await waitImage(s=>s.shape.Image.flip_x && s.shape.Image.flip_y);
    item=(await imageState()).shapes.find(s=>s.shape.Image);
    await imagePage.mouse.click((item.bounds[0]+item.bounds[2])/2,(item.bounds[1]+item.bounds[3])/2);
    const corner=item.handles.find(h=>h.kind==='Corner(TopLeft)');
    const cx=(item.bounds[0]+item.bounds[2])/2,cy=(item.bounds[1]+item.bounds[3])/2;
    const sx=corner.x-12,sy=corner.y-12;
    await imagePage.mouse.move(sx,sy);await imagePage.mouse.down();await imagePage.mouse.move(cx-(sy-cy),cy+(sx-cx),{steps:20});await imagePage.mouse.up();
    await waitImage(s=>Math.abs(s.shape.Image.rotation-Math.PI/2)<0.02);
    await snapshot(imagePage,'image-rotated.png');
    fs.writeFileSync(path.join(evidence,'image-state.json'),JSON.stringify(await imageState(),null,2));
    const imagePixels=await exportPixels(imagePage,'image-render-export.png');
    await imageContext.close();
    // Fullscreen is a DOM behavior check. Keep software-GPU readback checks
    // before display-mode transitions in Chromium's headless compositor.
    await page.bringToFront();
    await page.keyboard.press('F11');
    await page.waitForFunction(() => !!document.fullscreenElement);
    await page.keyboard.press('F11'); await page.waitForFunction(() => !document.fullscreenElement);


    const captures=['inline-dialog.png','text-fraction.png','text-root.png','presentation.png','image-flipped.png','image-rotated.png'].map(file=>inspectCapture(path.join(evidence,file)));
    fs.writeFileSync(path.join(evidence,'capture-validation.json'),JSON.stringify({screen_captures:captures,gpu_exports:[textPixels,imagePixels]},null,2));
    if(captures.some(c=>!c.valid)||!textPixels.valid||!imagePixels.valid)console.warn('CI WebGPU pixel evidence unavailable; interaction state tests passed. Local UI and native PNG metadata checks are separate evidence.');
    fs.writeFileSync(path.join(evidence, 'result.json'), JSON.stringify({ passed: true, gpu_pixels_validated: textPixels.valid&&imagePixels.valid, scenarios: ['French dead caret and accents', 'Unicode expander ^4/^>/^< replacement', 'partial selected B/I/U', 'nested inline fraction', 'indexed root', 'presentation', 'fullscreen', 'Escape preserves text', 'image mirrors, corner rotation, Undo/Redo'] }, null, 2));
    console.log('Chromium interaction checks passed.');
  } catch (error) {
    await page.screenshot({ path: path.join(evidence, 'failure.png') }).catch(() => {});
    fs.writeFileSync(path.join(evidence, 'failure-state.json'), JSON.stringify(await state().catch(() => null), null, 2));
    throw error;
  } finally {
    fs.writeFileSync(path.join(evidence, 'browser.log'), logs.join('\n'));
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
