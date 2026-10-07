// Real Chromium/WASM input; read-only diagnostics never mutate the application.
const { chromium } = require('../work/e2e/node_modules/playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const evidence = path.resolve('work/e2e/evidence');
fs.mkdirSync(evidence, { recursive: true });

(async () => {
  const browser = await chromium.launch({ headless: true, args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader', '--disable-vulkan-surface'] });
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
  const control = async name => {
    const s = await wait(s => !!s.controls[name]);
    const [x0, y0, x1, y1] = s.controls[name];
    await page.mouse.click((x0 + x1) / 2, (y0 + y1) / 2);
  };
  const fill = async (name, value) => { await control(name); await page.keyboard.press('Control+a'); await keys(value); };
  try {
    await page.goto(process.env.DRAFFTINK_TEST_URL || 'http://127.0.0.1:8888/?drafftink-test=1');
    await wait(s => s.shapes.length === 0);
    await page.keyboard.press('t');
    await page.mouse.click(370, 270);
    await wait(s => !!s.editing_text);
    await keys('123^4');
    await wait(s => text(s)?.content === '123^4');
    await page.keyboard.press('Backspace'); await page.keyboard.press('Backspace'); await keys('⁴');
    await wait(s => text(s)?.content === '123⁴');
    await keys('^^^p≥≤');
    await wait(s => text(s)?.content === '123⁴^^^p≥≤');
    await page.keyboard.press('Control+a');
    await page.keyboard.press('Control+b'); await page.keyboard.press('Control+i'); await page.keyboard.press('Control+u');
    await wait(s => text(s)?.char_styles.length === Array.from(text(s).content).length && text(s).char_styles.every(style => style.bold && style.italic && style.underline));
    await page.keyboard.press('ArrowRight');
    await control('Fraction'); await fill('Numérateur', '1/2'); await fill('Dénominateur', '3/4'); await control('Insérer');
    await wait(s => text(s)?.formulas.length === 1 && !s.inline_dialog);
    await page.screenshot({ path: path.join(evidence, 'text-fraction.png') });
    await keys(' fin');
    await wait(s => text(s)?.content.endsWith(' fin'));
    await control('Racine n-ième'); await fill('Expression', 'x+1'); await fill('Indice de la racine', '3'); await control('Insérer');
    await wait(s => text(s)?.formulas.length === 2 && !s.inline_dialog);
    await page.screenshot({ path: path.join(evidence, 'text-root.png') });
    const before = await state();
    await page.keyboard.press('Control+p'); await wait(s => s.presentation);
    await page.screenshot({ path: path.join(evidence, 'presentation.png') });
    await page.keyboard.press('Control+p'); await wait(s => !s.presentation);
    assert.equal(text(await state()).content, text(before).content);
    await page.keyboard.press('F11');
    await page.waitForFunction(() => !!document.fullscreenElement);
    await page.keyboard.press('F11'); await page.waitForFunction(() => !document.fullscreenElement);
    await page.keyboard.press('Escape');
    await wait(s => !s.editing_text && text(s)?.formulas.length === 2);
    assert(!logs.some(line => line.startsWith('PAGEERROR:')), logs.join('\n'));
    fs.writeFileSync(path.join(evidence, 'state.json'), JSON.stringify(await state(), null, 2));
    fs.writeFileSync(path.join(evidence, 'result.json'), JSON.stringify({ passed: true, scenarios: ['literal caret', 'Unicode expander replacement', 'selected B/I/U', 'nested inline fraction', 'indexed root', 'presentation', 'fullscreen', 'Escape preserves text'] }, null, 2));
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
