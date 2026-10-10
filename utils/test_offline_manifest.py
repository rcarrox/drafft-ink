import tempfile, unittest
from pathlib import Path
from build_offline_cache import build, FILES

class ManifestTests(unittest.TestCase):
    def test_html_line_endings_are_transport_safe_but_content_is_not(self):
        scratch=Path('work/offline-manifest-tests').resolve()
        scratch.mkdir(parents=True,exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as folder:
            root=Path(folder)
            for name in FILES:
                f=root/name;f.parent.mkdir(parents=True,exist_ok=True)
                f.write_bytes(b'\x00asm\x01\x00\x00\x00payload' if name.endswith('.wasm') else name.encode())
            (root/'index.html').write_bytes(b'<html>\n<title>Qurso</title>\n</html>\n')
            (root/'sw.template.js').write_text('const BUILD=__Q_CACHE_BUILD__;const ASSETS=__Q_CACHE_ASSETS__;',encoding='utf-8')
            expected=build(root)
            (root/'index.html').write_bytes(b'<html>\r\n<title>Qurso</title>\r\n</html>\r\n')
            self.assertEqual(build(root),expected)
            (root/'index.html').write_bytes(b'<html>\r\n<title>Different page</title>\r\n</html>\r\n')
            self.assertNotEqual(build(root),expected)

    def test_reproducible_build_and_changed_wasm_revision(self):
        scratch=Path('work/offline-manifest-tests').resolve()
        scratch.mkdir(parents=True,exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as folder:
            root=Path(folder)
            for name in FILES:
                f=root/name;f.parent.mkdir(parents=True,exist_ok=True)
                f.write_bytes(b'\x00asm\x01\x00\x00\x00payload' if name.endswith('.wasm') else name.encode())
            (root/'sw.template.js').write_text('const BUILD=__Q_CACHE_BUILD__;const ASSETS=__Q_CACHE_ASSETS__;',encoding='utf-8')
            before=build(root);generated=(root/'sw.js').read_text(encoding='utf-8')
            self.assertNotIn('__Q_CACHE',generated)
            self.assertEqual(build(root),before)
            self.assertEqual((root/'sw.js').read_text(encoding='utf-8'),generated)
            wasm=root/'pkg/drafftink_app_bg.wasm';wasm.write_bytes(wasm.read_bytes()+b'new release')
            self.assertNotEqual(build(root),before)
            wasm.write_bytes(b'host fallback HTML')
            with self.assertRaises(AssertionError):build(root)
            wasm.unlink()
            with self.assertRaises(FileNotFoundError):build(root)

if __name__=='__main__':unittest.main()
