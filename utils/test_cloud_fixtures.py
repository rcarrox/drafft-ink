"""Portable fixture checks; no private font or Windows session is needed."""
import importlib.util, json, struct, tempfile, zlib
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('generator',ROOT/'utils/generate_cloud_fixtures.py')
generator=importlib.util.module_from_spec(spec);spec.loader.exec_module(generator)

with tempfile.TemporaryDirectory() as scratch:
    out=Path(scratch);generator.generate(out)
    png=(out/'image-1200x2000.png').read_bytes()
    assert png[:8]==b'\x89PNG\r\n\x1a\n'
    assert struct.unpack_from('>II',png,16)==(1200,2000)
    offset=8;compressed=bytearray()
    while offset<len(png):
        size=struct.unpack_from('>I',png,offset)[0];kind=png[offset+4:offset+8];data=png[offset+8:offset+8+size]
        crc=struct.unpack_from('>I',png,offset+8+size)[0]
        assert zlib.crc32(kind+data)&0xffffffff==crc
        if kind==b'IDAT':compressed.extend(data)
        offset+=12+size
    assert len(zlib.decompress(compressed))==2000*(1+1200*4)
    manifest=json.loads((out/'manifest.json').read_text())
    assert manifest['decoded_rgba_bytes']==9_600_000 and manifest['private_google_font_included'] is False
    assert (ROOT/manifest['font_source']).is_file()
cases=json.loads((ROOT/'tests/fixtures/keyboard-fr.json').read_text())['cases']
assert len(cases)==7 and any(case['name']=='french_dead_caret' for case in cases)
assert all('expected' in case or 'expected_document_formula' in case for case in cases)
print('Cloud fixture checks passed: deterministic PNG/CRC/size, public font source, keyboard scenarios.')
