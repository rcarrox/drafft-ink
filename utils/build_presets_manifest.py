"""Index PNG presets for static FTP hosting (no directory listing required)."""
import json
from pathlib import Path


def build(root=Path('web/presets')):
    root.mkdir(parents=True, exist_ok=True)
    files = sorted(p.name for p in root.iterdir() if p.is_file() and p.suffix.lower() == '.png')
    (root / 'manifest.json').write_text(json.dumps({'files': files}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    return files


if __name__ == '__main__':
    build()
