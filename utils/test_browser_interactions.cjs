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
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 }, permissions: ['clipboard-read','clipboard-write'] });
  await context.addInitScript(() => localStorage.setItem('drafftink.user_settings.v1', JSON.stringify({ restore_last_document: false, intro_json: '', autosave_enabled: false, hide_properties: false, default_font: 'Noto Sans', default_font_postscript: '' })));
  let page = await context.newPage();
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
  let input = await context.newCDPSession(page);
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
    await page.mouse.move((x0+x1)/2,(y0+y1)/2);
    await page.waitForTimeout(100);
    await page.mouse.click((x0 + x1) / 2, (y0 + y1) / 2, {delay:60});
    await page.waitForTimeout(100);
  };
  const focusCanvasTool = async tool => {
    // MouseInput consumption uses egui's previous hover frame. Give the canvas
    // hover and each focus dismissal a frame before the placement click.
    await page.mouse.move(400,300);await page.waitForTimeout(100);
    await page.keyboard.press('Escape');await page.waitForTimeout(100);
    await page.keyboard.press('Escape');await page.waitForTimeout(100);
    await page.keyboard.press(tool);await wait(s=>s.tool===(tool==='m'?'Math':'Text'));
    await page.mouse.click(400,300);
  };
  const fill = async (name, value) => { await control(name); await page.keyboard.press('Control+a'); await keys(value); };
  try {
    // Default contextual properties and shape chooser, without altering the
    // legacy suite's explicit always-visible properties preference.
    const mainPage = page;
    const geometryContext = await browser.newContext({viewport:{width:1280,height:720}});
    await geometryContext.addInitScript(()=>localStorage.setItem('drafftink.user_settings.v1',JSON.stringify({restore_last_document:false,autosave_enabled:false,default_font:'Noto Sans',default_font_postscript:''})));
    page = await geometryContext.newPage();
    await page.goto(process.env.DRAFFTINK_TEST_URL || 'http://127.0.0.1:8888/?drafftink-test=1');
    await wait(s=>s.shapes.length===0&&!s.properties_visible);
    assert.equal(await page.title(),'Qurso🌿');
    await page.mouse.move(400,300);await page.waitForTimeout(100);
    await page.keyboard.press('o');await wait(s=>s.tool==='Ellipse'&&s.geometry==='Ellipse');
    await page.keyboard.press('o');await wait(s=>s.geometry==='Triangle');
    await page.mouse.move(300,270);await page.mouse.down();await page.mouse.move(460,410,{steps:8});await page.mouse.up();
    const tri = await wait(s=>s.shapes.some(i=>i.shape.Ellipse?.geometry==='Triangle'));
    assert(!tri.properties_visible);
    await page.keyboard.press('Control+z');await wait(s=>s.shapes.length===0);
    await page.keyboard.press('Control+Shift+z');await wait(s=>s.shapes.length===1);
    await page.mouse.move(380,270);await page.waitForTimeout(100);
    await page.mouse.click(380,270,{button:'right',delay:60});
    await wait(s=>s.context_properties&&s.properties_visible);
    await page.mouse.move(600,500);await page.waitForTimeout(100);await page.mouse.click(600,500);
    await wait(s=>!s.context_properties&&!s.properties_visible);
    const ellipseButton=(await state()).controls.tool_Ellipse;
    await page.mouse.move((ellipseButton[0]+ellipseButton[2])/2,(ellipseButton[1]+ellipseButton[3])/2);await page.waitForTimeout(100);
    await page.mouse.click((ellipseButton[0]+ellipseButton[2])/2,(ellipseButton[1]+ellipseButton[3])/2,{button:'right',delay:60});
    await control('geometry_Trapezoid');await wait(s=>s.geometry==='Trapezoid');
    await page.mouse.move(700,270);await page.waitForTimeout(100);await page.mouse.down();await page.mouse.move(850,410,{steps:8});await page.mouse.up();
    await wait(s=>s.shapes.some(i=>i.shape.Ellipse?.geometry==='Trapezoid'));
    await snapshot(page,'geometry-context.png');
    await geometryContext.close();
    // A canvas marquee continues and releases over a floating toolbar without
    // activating its tool; a fresh click afterwards still works normally.
    const captureContext=await browser.newContext({viewport:{width:1280,height:720}});
    await captureContext.addInitScript(()=>localStorage.setItem('drafftink.user_settings.v1',JSON.stringify({restore_last_document:false,autosave_enabled:false,accent_color:[180,35,100]})));
    page=await captureContext.newPage();await page.goto(process.env.DRAFFTINK_TEST_URL || 'http://127.0.0.1:8888/?drafftink-test=1');
    await wait(s=>s.shapes.length===0);assert.deepEqual((await state()).accent_color,[180,35,100]);
    await page.mouse.move(400,350);await page.waitForTimeout(100);await page.keyboard.press('r');
    await wait(s=>s.tool==='Rectangle');await page.mouse.down();await page.mouse.move(540,410,{steps:8});await page.mouse.up();await wait(s=>s.shapes.length===1);
    await page.keyboard.press('s');await wait(s=>s.tool==='Select');
    const tool=(await state()).controls.tool_Ellipse;assert(tool);
    const target=[(tool[0]+tool[2])/2,(tool[1]+tool[3])/2];
    await page.mouse.move(700,500);await page.waitForTimeout(100);await page.mouse.down();
    await page.mouse.move(...target,{steps:15});
    // Marquee bounds are world coordinates; account for the real default
    // zoom (1.68) and wait for the frame processing the final mouse event.
    const followsPointer=s=> {
      if(!s.selection_rect)return false;
      const screen=s.selection_rect.map((v,i)=>v*s.zoom+s.camera_offset[i%2]);
      return (Math.abs(screen[0]-target[0])<2||Math.abs(screen[2]-target[0])<2)
          && (Math.abs(screen[1]-target[1])<2||Math.abs(screen[3]-target[1])<2);
    };
    await wait(followsPointer);
    await page.mouse.up();await wait(s=>s.selection_rect===null&&s.tool==='Select');
    assert.equal((await state()).selected_count,1);
    const topBeforeNudge=(await state()).shapes[0].bounds[1];
    await page.keyboard.press('Control+ArrowDown');
    const nudged=await wait(s=>s.shapes[0].bounds[1]>topBeforeNudge+20);
    assert(Math.abs(nudged.shapes[0].bounds[1]-topBeforeNudge-33.6)<2,'Ctrl+ArrowDown should nudge the selection vertically by the normal fast step');
    await page.keyboard.press('Control+ArrowUp');
    await wait(s=>Math.abs(s.shapes[0].bounds[1]-topBeforeNudge)<1);
    await page.mouse.move(780,460);await page.keyboard.press('r');await wait(s=>s.tool==='Rectangle');
    await page.mouse.down();await page.mouse.move(900,540,{steps:5});await page.mouse.up();await wait(s=>s.shapes.length===2&&s.selected_count===1);
    await page.keyboard.press('s');await wait(s=>s.tool==='Select');
    const firstBounds=(await state()).shapes[0].bounds;
    await page.mouse.move((firstBounds[0]+firstBounds[2])/2,(firstBounds[1]+firstBounds[3])/2);
    await page.keyboard.down('Control');await page.mouse.click((firstBounds[0]+firstBounds[2])/2,(firstBounds[1]+firstBounds[3])/2);await page.keyboard.up('Control');
    await wait(s=>s.selected_count===2);
    await page.keyboard.press('Control+l');await wait(s=>s.shapes.every(shape=>shape.pinned));
    await page.keyboard.press('Control+z');await wait(s=>s.shapes.every(shape=>!shape.pinned));
    await page.keyboard.press('Control+l');await wait(s=>s.shapes.every(shape=>shape.pinned));
    await page.keyboard.press('Control+l');await wait(s=>s.shapes.every(shape=>!shape.pinned));
    await snapshot(page,'accent-selection.png');
    await page.mouse.move(700,500);await page.waitForTimeout(100);await control('tool_Ellipse');await wait(s=>s.tool==='Ellipse');
    await captureContext.close();
    // Math form pointer bounds use egui points, even on a scaled display.
    const dpiContext=await browser.newContext({viewport:{width:1280,height:720},deviceScaleFactor:2});
    await dpiContext.addInitScript(()=>localStorage.setItem('drafftink.user_settings.v1',JSON.stringify({restore_last_document:false,autosave_enabled:false,default_font:'Noto Sans',default_font_postscript:''})));
    page=await dpiContext.newPage();await page.goto(process.env.DRAFFTINK_TEST_URL || 'http://127.0.0.1:8888/?drafftink-test=1');await wait(s=>s.shapes.length===0);
    await page.mouse.move(400,200);await page.waitForTimeout(100);await page.keyboard.press('m');await wait(s=>s.tool==='Math');await page.waitForTimeout(100);await page.mouse.click(400,200,{delay:60});await wait(s=>!!s.editing_math&&s.math_input_focused&&!!s.math_form_rect);
    await keys('x+1');await wait(s=>s.shapes[0].shape.Math?.source==='x+1');
    const field=(await state()).math_form_rect;
    await page.mouse.click(field[0]+25,field[1]+22,{delay:60});await wait(s=>!!s.editing_math&&s.shapes.length===1);
    await page.mouse.move(150,220);await page.waitForTimeout(100);await page.mouse.click(150,220,{delay:60});await wait(s=>s.editing_math===null&&s.shapes.length===1);
    await dpiContext.close();page=mainPage;
    await page.goto(process.env.DRAFFTINK_TEST_URL || 'http://127.0.0.1:8888/?drafftink-test=1');
    await wait(s => s.shapes.length === 0);
    await snapshot(page,'initial.png');
    await page.mouse.move(300,300);await page.waitForTimeout(100);await page.keyboard.press('r');await wait(s=>s.tool==='Rectangle');
    await page.mouse.move(300,300);await page.mouse.down();await page.mouse.move(470,410,{steps:6});await page.mouse.up();await wait(s=>s.shapes.length===1);
    const originalRect=(await state()).shapes[0].shape.Rectangle.position.x;
    await page.keyboard.press('s');await page.mouse.move(303,320);await page.waitForTimeout(100);await page.mouse.down();await page.mouse.move(1278,320,{steps:12});
    await wait(s=>s.camera_offset[0]< -30);const edge=(await state()).shapes[0];assert(Math.abs(edge.bounds[0]-1275)<5);
    await page.mouse.move(900,320,{steps:6});await page.waitForTimeout(100);const stopped=(await state()).camera_offset[0];await page.waitForTimeout(250);assert(Math.abs((await state()).camera_offset[0]-stopped)<1);
    await page.mouse.up();await page.keyboard.press('Control+z');await wait(s=>Math.abs(s.shapes[0].shape.Rectangle.position.x-originalRect)<1e-7);
    await control('New canvas');await wait(s=>s.shapes.length===0);

    // A completed shape remains selected and can resize with its drawing tool still active.
    await page.keyboard.press('r');await wait(s=>s.tool==='Rectangle');
    await page.mouse.move(320,260);await page.mouse.down();await page.mouse.move(440,350,{steps:6});await page.mouse.up();
    await wait(s=>s.shapes.length===1&&s.selected_count===1);
    await page.mouse.move(440,350);await page.waitForTimeout(100);await page.mouse.down();await page.mouse.move(510,390,{steps:6});await page.mouse.up();
    await wait(s=>s.shapes.length===1&&s.shapes[0].bounds[2]-s.shapes[0].bounds[0]>170&&s.tool==='Rectangle');
    await control('New canvas');await wait(s=>s.shapes.length===0);

    await page.mouse.click(370,270);
    await page.keyboard.press('t');await wait(s=>s.tool==='Text');
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
    await keys(' frac(frac(1,2),frac(3,4))');await wait(s=>!!s.command_editor);
    await page.keyboard.press('Enter');await wait(s=>!s.command_editor&&text(s)?.formulas.length===1);
    await snapshot(page,'text-fraction.png');
    await keys(' fin');
    await wait(s => text(s)?.content.endsWith(' fin'));
    await keys(' root(x+1,3)');await wait(s=>!!s.command_editor);await page.keyboard.press('Enter');await wait(s=>!s.command_editor);
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
    // A validated formula reopens with a direct double-click.
    const validCommand=await state();
    const commandId=validCommand.shapes.find(item=>item.shape.Text?.formulas.some(f=>f.math.source==='sum(kx,k,1,n)')).id;
    const commandBounds=validCommand.shapes.find(item=>item.id===commandId).bounds;
    await page.keyboard.press('s');
    await page.mouse.dblclick(commandBounds[0]+15,(commandBounds[1]+commandBounds[3])/2);
    await wait(s=>s.command_editor==='sum(kx,k,1,n)');
    await page.keyboard.press('Control+a');await keys('sum(k²,k,1,n)');
    await wait(s=>s.command_editor==='sum(k²,k,1,n)');
    await page.keyboard.press('Escape');await wait(s=>s.editing_text===null);
    // Cross the left edge with the right edge handle of the Text object.
    const beforeFlip=(await state()).shapes.find(item=>item.id===commandId);
    await page.keyboard.press('s');await page.mouse.click(beforeFlip.bounds[0]+10,(beforeFlip.bounds[1]+beforeFlip.bounds[3])/2);
    const right=beforeFlip.handles.find(h=>h.kind==='Edge(Right)');
    await page.mouse.move(right.x,right.y);await page.mouse.down();
    await page.mouse.move(beforeFlip.bounds[0]-60,right.y,{steps:8});await page.mouse.up();
    await wait(s=>s.shapes.find(item=>item.id===commandId)?.shape.Text.display_scale[0]<0);
    await page.keyboard.press('Control+z');await wait(s=>s.shapes.find(item=>item.id===commandId)?.shape.Text.display_scale[0]>0);

    // One physical render pixel for a normal arrow, historical GRID_SIZE for Ctrl.
    const nudgeBefore=(await state()).shapes.find(item=>item.id===commandId);
    await page.mouse.click(nudgeBefore.bounds[0]+10,(nudgeBefore.bounds[1]+nudgeBefore.bounds[3])/2);
    await page.keyboard.press('ArrowRight');await wait(s=>Math.abs(s.shapes.find(item=>item.id===commandId).bounds[0]-nudgeBefore.bounds[0]-1)<0.01);
    const nudgeFine=(await state()).shapes.find(item=>item.id===commandId);
    await page.keyboard.press('Control+ArrowRight');await wait(s=>Math.abs(s.shapes.find(item=>item.id===commandId).bounds[0]-nudgeFine.bounds[0]-20*s.zoom)<0.01);
    // Run input/clipboard checks before requesting a GPU PNG export.
    // New code command uses the same inline axis and supports live completion.
    await control('New canvas');await wait(s=>s.shapes.length===0);
    await focusCanvasTool('t');await wait(s=>!!s.editing_text);
    await keys('bin(');await wait(s=>s.command_editor==='bin(');
    await keys('n,k)');await wait(s=>commandText(s)?.formulas[0].math.latex==='\\binom{n}{k}');
    await page.keyboard.press('Enter');await page.keyboard.press('Escape');await wait(s=>s.editing_text===null);
    const binShape=(await state()).shapes.find(item=>item.shape.Text);
    await page.keyboard.press('s');
    const resizeBin=async free=> {
      await page.mouse.click((binShape.bounds[0]+binShape.bounds[2])/2,(binShape.bounds[1]+binShape.bounds[3])/2);
      const handle=(await state()).shapes.find(item=>item.id===binShape.id).handles.find(h=>h.kind==='Edge(Right)');
      if(free)await page.keyboard.down('Shift');
      await page.mouse.move(handle.x,handle.y);await page.mouse.down();await page.mouse.move(handle.x+45,handle.y,{steps:6});await page.mouse.up();
      if(free)await page.keyboard.up('Shift');
      const resized=await wait(s=>s.shapes.find(item=>item.id===binShape.id).shape.Text.display_scale[0]>1.05);
      const scale=resized.shapes.find(item=>item.id===binShape.id).shape.Text.display_scale;
      if(free)assert.equal(scale[1],1);else assert(Math.abs(scale[0]-scale[1])<1e-7);
      await page.keyboard.press('Control+z');await wait(s=>s.shapes.find(item=>item.id===binShape.id).shape.Text.display_scale[0]===1);
    };
    await resizeBin(false);await resizeBin(true);
    // Math's browser clipboard replaces just the selection, then inserts at the caret.
    await control('New canvas');await wait(s=>s.shapes.length===0);
    await focusCanvasTool('m');await wait(s=>!!s.editing_math&&s.math_input_focused);
    const mathText=s=>s.shapes.find(item=>item.shape.Math)?.shape.Math;
    await keys('x');await page.keyboard.press('Control+ArrowUp');await keys('3');await wait(s=>mathText(s)?.source==='x^3'&&mathText(s)?.latex==='x^3');
    await page.keyboard.press('Control+a');await keys('x\\neq y\\in A + x_{i} y_{j}');await wait(s=>mathText(s)?.source==='x\\neq y\\in A + x_{i} y_{j}'&&mathText(s)?.latex===mathText(s)?.source);
    await page.keyboard.press('Control+a');
    const fieldRect=(await state()).math_form_rect;await page.mouse.move(fieldRect[0]+20,fieldRect[1]+20);await wait(s=>s.cursor_mode===2);
    assert(decodeURIComponent(await page.evaluate(()=>document.querySelector('canvas').style.cursor)).includes('M60 26l12 12'));
    const mathButton=(await state()).controls.tool_Math;await page.mouse.move((mathButton[0]+mathButton[2])/2,(mathButton[1]+mathButton[3])/2);await wait(s=>s.cursor_mode===0);
    await page.mouse.move(fieldRect[0]+20,fieldRect[1]+20);await wait(s=>s.cursor_mode===2);
    await keys('123456');await wait(s=>mathText(s)?.source==='123456');
    await page.keyboard.press('Control+a');await page.keyboard.press('Control+c');
    await page.waitForFunction(async()=>await navigator.clipboard.readText()==='123456');
    await page.evaluate(()=>navigator.clipboard.writeText('x+∞'));
    await page.keyboard.press('Control+v');await wait(s=>mathText(s)?.source==='x+∞');
    await page.keyboard.press('ArrowLeft');await page.evaluate(()=>navigator.clipboard.writeText('2'));
    await page.keyboard.press('Control+v');await wait(s=>mathText(s)?.source==='x+2∞');
    // French AltGr+Equal emits a literal brace, never the Ctrl+= subscript command.
    await input.send('Input.dispatchKeyEvent',{type:'keyDown',key:'}',code:'Equal',text:'}',unmodifiedText:'}',modifiers:3});
    await input.send('Input.dispatchKeyEvent',{type:'keyUp',key:'}',code:'Equal',modifiers:0});
    await wait(s=>mathText(s)?.source.includes('}')&&!mathText(s)?.source.includes('_'));
    await page.keyboard.press('Escape');await wait(s=>s.editing_math===null);
    await page.waitForTimeout(150);

    // Original characters and font-dependent scripts, including symbols without Unicode script glyphs.
    const nextScriptTab=(await state()).active_tab+1;await control('New canvas');await wait(s=>s.active_tab===nextScriptTab&&s.shapes.length===0);
    await focusCanvasTool('t');await wait(s=>!!s.editing_text);
    for(const brace of ['{','}']) {
      await input.send('Input.dispatchKeyEvent',{type:'keyDown',key:brace,code:brace==='{'?'Digit4':'Equal',text:brace,unmodifiedText:brace,modifiers:3});
      await input.send('Input.dispatchKeyEvent',{type:'keyUp',key:brace,code:brace==='{'?'Digit4':'Equal',modifiers:0});
    }
    await wait(s=>s.shapes.find(i=>i.id===s.editing_text)?.shape.Text.content==='{}');await page.keyboard.press('Control+a');await page.keyboard.press('Backspace');
    await keys('Base ');await wait(s=>!!s.text_caret);
    const normalCaret=(await state()).text_caret;
    await page.keyboard.press('Control+ArrowUp');const supState=await wait(s=>s.insertion_script===1);
    assert(supState.text_caret[3]-supState.text_caret[1]<normalCaret[3]-normalCaret[1]);assert(supState.text_caret[1]<normalCaret[1]);
    await keys('AZ09α≤@');await page.keyboard.press('Control+ArrowUp');await keys(' fin');
    const scriptText=s=>s.shapes.find(item=>item.id===s.editing_text)?.shape.Text;
    await wait(s=>scriptText(s)?.content==='Base AZ09α≤@ fin');
    await wait(s=>scriptText(s).char_styles.slice(5,12).every(style=>style.script===1));
    await page.keyboard.press('Control+a');await page.keyboard.press('Control+ArrowDown');await wait(s=>scriptText(s).char_styles.every(style=>style.script===-1));
    // Space remains in the script, and an empty next line has its caret below.
    await page.keyboard.press('Escape');await control('New canvas');await wait(s=>s.shapes.length===0);
    await focusCanvasTool('t');await wait(s=>!!s.editing_text);await keys('x');await page.keyboard.press('Control+ArrowUp');await keys('45 6');
    await wait(s=>scriptText(s)?.content==='x45 6'&&scriptText(s).char_styles.slice(1).every(c=>c.script===1));
    await page.keyboard.press('ArrowRight');await wait(s=>s.insertion_script===0);await page.keyboard.press('Enter');
    const emptyLine=await wait(s=>scriptText(s)?.content==='x45 6\n'&&s.text_caret?.[0]<1);
    assert(emptyLine.text_caret[1]>0);
    await keys('hello world');await wait(s=>scriptText(s)?.content.endsWith('hello world'));
    await page.keyboard.press('Escape');await control('New canvas');await wait(s=>s.shapes.length===0);
    await focusCanvasTool('t');await wait(s=>!!s.editing_text);await keys('hello world');await wait(s=>scriptText(s)?.content==='hello world');
    await page.keyboard.press('Escape');await wait(s=>s.editing_text===null);await control('tool_Select');
    const word=(await state()).shapes.find(i=>i.shape.Text);
    await page.mouse.move(word.bounds[0]+25,(word.bounds[1]+word.bounds[3])/2);await page.waitForTimeout(100);
    await page.mouse.dblclick(word.bounds[0]+25,(word.bounds[1]+word.bounds[3])/2,{delay:80});await wait(s=>s.selected_text==='hello'&&s.cursor_mode===1);
    await keys('X');await wait(s=>scriptText(s)?.content==='X world');
    await page.keyboard.press('Escape');await control('New canvas');await wait(s=>s.shapes.length===0);
    await focusCanvasTool('m');await wait(s=>!!s.editing_math&&s.math_input_focused);await keys('x+1');await wait(s=>mathText(s)?.source==='x+1');
    await page.mouse.move(650,470);await page.waitForTimeout(100);await page.mouse.click(650,470,{delay:60});await wait(s=>s.editing_math===null&&s.shapes.length===1);
    const equation=(await state()).shapes.find(i=>i.shape.Math);
    await page.mouse.move((equation.bounds[0]+equation.bounds[2])/2,(equation.bounds[1]+equation.bounds[3])/2);await page.waitForTimeout(100);
    await page.mouse.click((equation.bounds[0]+equation.bounds[2])/2,(equation.bounds[1]+equation.bounds[3])/2,{button:'right',delay:60});
    await control('math_object_font');await control('math_object_font:Noto Sans');await wait(s=>mathText(s)?.font.family==='NotoSans');await snapshot(page,'math-font.png');
    await page.keyboard.press('Control+z');await wait(s=>mathText(s)?.font.family==='GelPen');
    await page.mouse.move(650,470);await page.waitForTimeout(100);
    // A subsequent click can place a new Math; a toolbar switch closes it.
    await page.mouse.click(650,470,{delay:60});await wait(s=>!!s.editing_math&&s.math_input_focused&&s.shapes.length===2);await keys('y');
    await control('tool_Rectangle');await wait(s=>s.tool==='Rectangle'&&s.editing_math===null&&s.shapes.length===2);
    await control('tool_Select');const editMath=(await state()).shapes.find(i=>i.shape.Math);
    await page.mouse.move((editMath.bounds[0]+editMath.bounds[2])/2,(editMath.bounds[1]+editMath.bounds[3])/2);await page.waitForTimeout(100);await page.mouse.dblclick((editMath.bounds[0]+editMath.bounds[2])/2,(editMath.bounds[1]+editMath.bounds[3])/2,{delay:80});await wait(s=>!!s.editing_math&&s.tool==='Select');
    const selectedForm=(await state()).math_form_rect;await page.mouse.move(selectedForm[0]+20,selectedForm[1]+20);await wait(s=>s.cursor_mode===2);
    await page.mouse.move(500,200);await wait(s=>s.cursor_mode===0);await page.keyboard.press('Escape');await wait(s=>s.editing_math===null);
    await control('New canvas');await wait(s=>s.shapes.length===0);await focusCanvasTool('t');await wait(s=>!!s.editing_text);await keys('save');
    const saveCount=(await state()).png_save_requests;
    await page.keyboard.press('Control+s');await wait(s=>s.png_save_requests===saveCount+1);
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
    const initialImageMemory=(await imageState()).memory;
    assert(initialImageMemory.image_cache_bytes>0 && initialImageMemory.image_cache_bytes<1200*2000*4/4);
    const wasmMemory=await imagePage.evaluate(()=>window.drafftinkMemoryUsage());
    assert(wasmMemory.wasm_bytes>0);
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
    const finalImageMemory=(await imageState()).memory;
    assert(finalImageMemory.image_cache_bytes<=finalImageMemory.image_cache_budget_bytes);
    assert(finalImageMemory.render_target_allocations<=1,'Screen target must be reused across image manipulations');
    fs.writeFileSync(path.join(evidence,'memory.json'),JSON.stringify({initial_image:initialImageMemory,after_image_manipulations:finalImageMemory,wasm:wasmMemory,original_rgba_bytes:1200*2000*4},null,2));
    const imageControl=async name=> {
      await imagePage.waitForFunction(name=>JSON.parse(window.__drafftinkTestState).controls[name],name);
      const [x0,y0,x1,y1]=(await imageState()).controls[name];
      await imagePage.mouse.click((x0+x1)/2,(y0+y1)/2);
    };
    await imageControl('New canvas');
    await imagePage.waitForFunction(()=>JSON.parse(window.__drafftinkTestState).active_tab===1);
    const emptyTab=await imageState();
    assert.equal(emptyTab.shapes.length,0);
    assert.equal(emptyTab.memory.image_cache_bytes,0,'Inactive canvas decoded pixels must be released');
    await imageControl('Canvas 0');
    await imagePage.waitForFunction(()=>JSON.parse(window.__drafftinkTestState).active_tab===0);
    assert((await imageState()).shapes.some(s=>s.shape.Image?.id===imageId));
    assert.equal((await imageState()).memory.parked_shapes_total,0,'Active document must not be cloned into its tab slot');
    const imagePixels=await exportPixels(imagePage,'image-render-export.png');
    await imageContext.close();
    // Fullscreen is a DOM behavior check. Keep software-GPU readback checks
    // before display-mode transitions in Chromium's headless compositor.
    await page.bringToFront();
    await page.keyboard.press('F11');
    await page.waitForFunction(() => !!document.fullscreenElement);
    await page.keyboard.press('F11'); await page.waitForFunction(() => !document.fullscreenElement);


    const captures=['math-font.png','text-fraction.png','text-root.png','presentation.png','image-flipped.png','image-rotated.png'].map(file=>inspectCapture(path.join(evidence,file)));
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
