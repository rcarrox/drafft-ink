"""Serve a downloaded Actions WASM build and public font fixture on loopback."""
import argparse, json
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[2]
FONT = ROOT / 'crates/drafftink-render/assets/NotoSans-Regular.ttf'

class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT / 'web'), **kwargs)
    def do_GET(self):
        route = unquote(urlsplit(self.path).path)
        if route == '/local-fonts.json':
            data = json.dumps([{'family':'Cloud Test Sans','postscriptName':'CloudTestSans-Regular','style':'Regular','source':'public-test-fixture'}]).encode()
            self.send_response(200); self.send_header('Content-Type','application/json'); self.send_header('Content-Length',str(len(data))); self.end_headers(); self.wfile.write(data)
        elif route == '/local-font/CloudTestSans-Regular':
            data = FONT.read_bytes()
            self.send_response(200); self.send_header('Content-Type','application/octet-stream'); self.send_header('Content-Length',str(len(data))); self.end_headers(); self.wfile.write(data)
        elif route.startswith('/local-font/'):
            self.send_error(404, 'Private fonts are not development fixtures')
        else:
            super().do_GET()

if __name__ == '__main__':
    parser=argparse.ArgumentParser(); parser.add_argument('--port',type=int,default=8888); args=parser.parse_args()
    required=[ROOT/'web/pkg/drafftink_app.js',ROOT/'web/pkg/drafftink_app_bg.wasm']
    if not all(p.is_file() for p in required):
        raise SystemExit('Download the matching drafftink-wasm Actions artifact into web/pkg first.')
    print(f'Public-fixture preview: http://127.0.0.1:{args.port}',flush=True)
    ThreadingHTTPServer(('127.0.0.1',args.port),Handler).serve_forever()
