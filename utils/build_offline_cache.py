"""Generate a scoped, integrity-checked offline app shell from one complete build."""
import argparse, base64, hashlib, json
from pathlib import Path

FILES = ['qlogo.svg', 'cursoreraser.svg', 'cursoreraserman.svg', 'cursorcrosshair.svg', 'cursordraw.svg', 'index.html', 'offline.js', 'favicon.svg', 'cursormouse.svg', 'cursortext.svg',
         'cursormath.svg', 'pkg/drafftink_app.js', 'pkg/drafftink_app_bg.wasm']

def build(root):
    root = Path(root)
    assets = []
    for name in FILES:
        payload = (root/name).read_bytes()
        assert payload, f'Empty asset: {name}'
        if name.endswith('.wasm'):
            assert payload[:8] == b'\x00asm\x01\x00\x00\x00', 'Invalid WASM build'
        assets.append({'path': name, 'integrity': 'sha256-'+base64.b64encode(hashlib.sha256(payload).digest()).decode()})
    template = (root/'sw.template.js').read_text(encoding='utf-8')
    encoded = json.dumps(assets, separators=(',', ':'))
    revision = hashlib.sha256((template+encoded).encode()).hexdigest()[:24]
    script = template.replace('__Q_CACHE_BUILD__', json.dumps(revision)).replace('__Q_CACHE_ASSETS__', encoded)
    (root/'sw.js').write_text(script, encoding='utf-8', newline='\n')
    return revision

if __name__ == '__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--web-dir',type=Path,default=Path('web'))
    print('Offline build:', build(parser.parse_args().web_dir))
