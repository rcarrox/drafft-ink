"""Generate public, deterministic fixtures without developer-PC dependencies."""
import argparse, hashlib, json, struct, zlib
from pathlib import Path

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)

def generate(out):
    out.mkdir(parents=True, exist_ok=True)
    width, height = 1200, 2000
    rows = bytearray()
    for y in range(height):
        rows.append(0)
        for x in range(width):
            tile = (x // 100 + y // 100) % 2
            rows.extend((40 + tile * 120, y * 255 // height, x * 255 // width, 255))
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
    png += chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b'')
    target = out / 'image-1200x2000.png'
    target.write_bytes(png)
    manifest = {'image': target.name, 'width': width, 'height': height, 'decoded_rgba_bytes': width * height * 4,
                'sha256': hashlib.sha256(png).hexdigest(), 'font': 'CloudTestSans-Regular',
                'font_source': 'crates/drafftink-render/assets/NotoSans-Regular.ttf',
                'private_google_font_included': False}
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(manifest))

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, default=Path('work/fixtures'))
    generate(parser.parse_args().out)
