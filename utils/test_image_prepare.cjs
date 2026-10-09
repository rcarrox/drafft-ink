const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');

const source = fs.readFileSync('web/offline.js', 'utf8');
const start = source.indexOf('// User presets live in IndexedDB');
const end = source.indexOf('window.qursoPlayAlarm', start);
assert(start >= 0 && end > start, 'offline image/preset helpers exist');
let bitmapClosed = false, drawArgs, blobArgs, canvasSize;
const sandbox = {
  window: {}, indexedDB: {}, crypto: {}, console,
  createImageBitmap: async () => ({width: 4000, height: 2000, close() { bitmapClosed = true; }}),
  document: {createElement: () => ({
    set width(value) { this._width = value; }, get width() { return this._width; },
    set height(value) { this._height = value; }, get height() { return this._height; },
    getContext: () => ({drawImage(...args) { drawArgs = args; }}),
    toBlob(callback, type, quality) { canvasSize = [this.width, this.height]; blobArgs = [type, quality]; callback({type}); },
  })},
};
vm.createContext(sandbox);
vm.runInContext(source.slice(start, end), sandbox);
(async () => {
  assert.equal(sandbox.window.qursoPresetDisplayName('trigo.png'), 'Trigo');
  assert.equal(sandbox.window.qursoPresetDisplayName('plan_complexe.png'), 'Plan Complexe');
  const prepared = await sandbox.window.qursoPrepareImage({type: 'image/jpeg'}, 2048);
  assert.deepEqual(Array.from([prepared.width, prepared.height]), [2048, 1024]);
  assert.deepEqual(canvasSize, [2048, 1024]);
  assert.deepEqual(blobArgs, ['image/jpeg', 0.88]);
  assert.equal(drawArgs.length, 5);
  assert(bitmapClosed, 'decoded source bitmap is released');
  const original = {type: 'image/png'};
  sandbox.createImageBitmap = async () => ({width: 1000, height: 800, close() { bitmapClosed = true; }});
  const unchanged = await sandbox.window.qursoPrepareImage(original, 2048);
  assert.equal(unchanged.blob, original, 'small transparent PNG is preserved byte-for-byte');
  console.log('Image import preparation scales large images and retains small originals.');
})().catch(error => { console.error(error); process.exitCode = 1; });
