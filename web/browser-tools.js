window.qursoClockNow = timeZone => {
    const options = {year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23'};
    if (timeZone && timeZone !== 'Local') options.timeZone = timeZone;
    const parts = Object.fromEntries(new Intl.DateTimeFormat('en-CA', options).formatToParts(new Date()).map(({type, value}) => [type, value]));
    return `${parts.year}-${parts.month}-${parts.day}|${parts.hour}:${parts.minute}:${parts.second}`;
};
// User presets live in IndexedDB so sizable transparent PNG diagrams do not
// consume localStorage quota. Their display names are independent of filenames.
(() => {
    const db = () => new Promise((resolve, reject) => {
        const request = indexedDB.open('qurso-presets', 1);
        request.onupgradeneeded = () => request.result.createObjectStore('presets', {keyPath: 'id'});
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
    const requestResult = request => new Promise((resolve, reject) => {
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
    const transaction = async (mode, operation) => {
        const database = await db();
        try {
            const tx = database.transaction('presets', mode);
            const completed = new Promise((resolve, reject) => { tx.oncomplete = resolve; tx.onerror = () => reject(tx.error); });
            const result = await requestResult(operation(tx.objectStore('presets')));
            await completed;
            return result;
        } finally { database.close(); }
    };
    const displayName = filename => filename.replace(/\.png$/i, '').replace(/[_-]+/g, ' ').trim().replace(/(^|\s)(\p{L})/gu, (_, space, letter) => space + letter.toUpperCase()) || 'Preset';
    window.qursoPresetDisplayName = displayName;
    let seedPromise;
    const seedKey = 'qurso.presets.seed.v1';
    const seedDefaults = async () => {
        if (localStorage.getItem(seedKey)) return;
        const response = await fetch(new URL('presets/manifest.json', document.baseURI), {cache: 'no-store'});
        if (!response.ok) throw new Error('Liste des presets indisponible');
        const manifest = await response.json();
        if (!Array.isArray(manifest.files)) throw new Error('Liste des presets invalide');
        for (const entry of manifest.files) {
            const file = typeof entry === 'string' ? entry : entry.file;
            if (typeof file !== 'string' || !/^[^/\\]+\.png$/i.test(file)) continue;
            const id = 'builtin:' + file;
            if (await transaction('readonly', store => store.get(id))) continue;
            const image = await fetch(new URL('presets/' + encodeURIComponent(file), document.baseURI));
            if (!image.ok) throw new Error('Preset manquant : ' + file);
            const blob = await image.blob();
            await transaction('readwrite', store => store.put({id, name: entry.name || displayName(file), blob}));
        }
        localStorage.setItem(seedKey, '1');
    };
    const ready = () => seedPromise ||= seedDefaults().catch(error => { console.warn('Presets prédéfinis :', error); });
    window.qursoPresetList = async () => { await ready(); return (await transaction('readonly', store => store.getAll())).map(({id, name}) => ({id, name})); };
    window.qursoPresetAdd = async () => {
        const file = await new Promise((resolve, reject) => {
            const input = document.createElement('input'); input.type = 'file'; input.multiple = true; input.accept = 'image/png,.png'; input.style.display = 'none';
            document.body.appendChild(input);
            input.onchange = () => { const selected = Array.from(input.files || []); input.remove(); selected ? resolve(selected) : resolve(null); };
            input.oncancel = () => { input.remove(); resolve(null); }; input.click();
        });
        if (!file?.length) return await window.qursoPresetList();
        for (const png of file) {
            if (!/\.png$/i.test(png.name)) continue;
            const id = crypto.randomUUID();
            await transaction('readwrite', store => store.put({id, name: displayName(png.name), blob: png}));
        }
        return await window.qursoPresetList();
    };
    window.qursoPresetRename = async (id, name) => {
        const database = await db();
        try {
            const tx = database.transaction('presets', 'readwrite'); const store = tx.objectStore('presets');
            const item = await requestResult(store.get(id)); if (item && name.trim()) { item.name = name.trim(); store.put(item); }
            await new Promise((resolve, reject) => { tx.oncomplete = resolve; tx.onerror = () => reject(tx.error); });
        } finally { database.close(); }
        return await window.qursoPresetList();
    };
    window.qursoPresetDelete = async id => { await transaction('readwrite', store => store.delete(id)); return await window.qursoPresetList(); };
    window.qursoPresetGet = async id => (await transaction('readonly', store => store.get(id)))?.blob || null;
    window.qursoPrepareImage = async (file, maxSide = 2048) => {
        const bitmap = await createImageBitmap(file);
        try {
            const scale = Math.min(1, maxSide / Math.max(bitmap.width, bitmap.height));
            if (scale === 1) return {blob: file, width: bitmap.width, height: bitmap.height};
            const canvas = document.createElement('canvas'); canvas.width = Math.max(1, Math.round(bitmap.width * scale)); canvas.height = Math.max(1, Math.round(bitmap.height * scale));
            canvas.getContext('2d', {alpha: true}).drawImage(bitmap, 0, 0, canvas.width, canvas.height);
            const type = file.type === 'image/jpeg' ? 'image/jpeg' : file.type === 'image/webp' ? 'image/webp' : 'image/png';
            const blob = await new Promise((resolve, reject) => canvas.toBlob(value => value ? resolve(value) : reject(new Error('Réduction de l’image impossible')), type, type === 'image/png' ? undefined : 0.88));
            canvas.width = canvas.height = 0;
            return {blob, width: bitmap.width * scale, height: bitmap.height * scale};
        } finally { bitmap.close(); }
    };
})();
window.qursoPlayAlarm = () => {
    try {
        const audio = new AudioContext();
        const oscillator = audio.createOscillator(); const gain = audio.createGain();
        oscillator.type = 'sine'; oscillator.frequency.value = 880; gain.gain.value = 0.12;
        oscillator.connect(gain); gain.connect(audio.destination); oscillator.start();
        gain.gain.setTargetAtTime(0, audio.currentTime + 0.35, 0.12);
        setTimeout(() => { oscillator.stop(); audio.close(); }, 900);
    } catch (error) { console.warn('Alarme audio indisponible', error); }
};

// Settings are portable JSON; browser folder permissions and private fonts remain local.
window.qursoPickSettings = () => new Promise(resolve => {
    const input = document.createElement('input'); input.type = 'file'; input.accept = '.json,application/json'; input.style.display = 'none';
    document.body.appendChild(input);
    input.onchange = async () => { const file = input.files?.[0]; input.remove(); resolve(file ? await file.text() : null); };
    input.oncancel = () => { input.remove(); resolve(null); }; input.click();
});
window.qursoOpenNumworks = () => window.open(new URL('numworks/', document.baseURI).href, '_blank', 'noopener,noreferrer');
