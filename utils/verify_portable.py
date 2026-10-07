"""Verify the inner Windows portable ZIP produced by GitHub Actions."""
import argparse, hashlib, json, zipfile
from pathlib import Path

def verify(path, version):
    required=['Lancer DrafftInk.cmd','Arreter DrafftInk.cmd','Ouvrir avec Edge.cmd','Ouvrir avec Chrome.cmd',
              'windows/serve-local.ps1','VERSION_LOCAL.txt','README_LOCAL_FR.md','web/index.html',
              'web/pkg/drafftink_app.js','web/pkg/drafftink_app_bg.wasm','web/cursormouse.svg','web/cursortext.svg']
    with zipfile.ZipFile(path) as z:
        assert z.testzip() is None, 'ZIP CRC failure'
        names={name.removeprefix('./'): name for name in z.namelist()}
        assert not set(required)-names.keys(), f'Missing files: {set(required)-names.keys()}'
        assert z.read(names['VERSION_LOCAL.txt']).decode('utf-8-sig').strip()==version, 'Version mismatch'
        wasm=z.read(names['web/pkg/drafftink_app_bg.wasm'])
        assert wasm[:8]==b'\x00asm\x01\x00\x00\x00', 'Invalid WASM header'
        assert len(wasm)>1_000_000 and b'drafftink_app_bg.wasm' in z.read(names['web/pkg/drafftink_app.js'])
        assert not any('googlesans' in name.lower() and name.lower().endswith(('.ttf','.otf','.ttc')) for name in names), 'Private font must not be packaged'
        sizes={name:z.getinfo(names[name]).file_size for name in required}
        assert all(sizes.values()), 'Empty required file'
    return {'version':version,'zip_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'zip_bytes':path.stat().st_size,
            'zip_crc':'OK','wasm_header':'OK','private_google_font_included':False,'files':sizes}

if __name__ == '__main__':
    parser=argparse.ArgumentParser();parser.add_argument('zip',type=Path);parser.add_argument('--version',required=True);parser.add_argument('--report',type=Path)
    args=parser.parse_args();report=verify(args.zip,args.version);text=json.dumps(report,indent=2)+'\n'
    if args.report: args.report.write_text(text,encoding='utf-8')
    print(text)
